/*
   Copyright 2023 huggingface/candle (Apache 2.0 / MIT)
   Copyright 2023 Krol Inventions B.V.

   This file is part of DawnSearch.

   DawnSearch is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published by
   the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   DawnSearch is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with DawnSearch.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::{sync::mpsc::Receiver, time::Instant};

use crate::{
    embedding::model::{BertModel, Config, DTYPE},
    search::vector::normalize,
};
use anyhow::{anyhow, Error as E, Result};
use candle::Tensor;
use candle_nn::VarBuilder;
use hf_hub::{api::sync::Api, Cache, Repo, RepoType};
use tokenizers::{PaddingParams, Tokenizer};

use candle::Device;

pub fn device(cpu: bool) -> Result<Device> {
    if cpu {
        Ok(Device::Cpu)
    } else {
        let device = Device::cuda_if_available(0)?;
        if !device.is_cuda() {
            println!("Running on CPU, to run on GPU, build this example with `--features cuda`");
        }
        Ok(device)
    }
}

struct EmbeddingProvider {
    tokenizer: Tokenizer,
    model: BertModel,
}

impl EmbeddingProvider {
    fn new() -> anyhow::Result<EmbeddingProvider> {
        let cpu = true;
        let offline = false;

        let device = device(cpu)?;
        let model_id = "sentence-transformers/all-MiniLM-L6-v2".to_string();
        let revision = "refs/pr/21".to_string();

        let repo = Repo::with_revision(model_id, RepoType::Model, revision);
        let (config_filename, tokenizer_filename, weights_filename) = if offline {
            let cache = Cache::default();
            (
                cache
                    .get(&repo, "config.json")
                    .ok_or(anyhow!("Missing config file in cache"))?,
                cache
                    .get(&repo, "tokenizer.json")
                    .ok_or(anyhow!("Missing tokenizer file in cache"))?,
                cache
                    .get(&repo, "model.safetensors")
                    .ok_or(anyhow!("Missing weights file in cache"))?,
            )
        } else {
            let api = Api::new()?;
            let api = api.repo(repo);
            (
                api.get("config.json")?,
                api.get("tokenizer.json")?,
                api.get("model.safetensors")?,
            )
        };
        let config = std::fs::read_to_string(config_filename)?;
        let config: Config = serde_json::from_str(&config)?;
        let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(E::msg)?;

        let weights = unsafe { candle::safetensors::MmapedFile::new(weights_filename)? };
        let weights = weights.deserialize()?;
        let vb = VarBuilder::from_safetensors(vec![weights], DTYPE, &device);
        let model = BertModel::load(vb, &config)?;
        Ok(EmbeddingProvider { model, tokenizer })
    }

    pub fn calculate_embedding(&mut self, inputs: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        let device = &self.model.device;

        let n_sentences = inputs.len();
        if let Some(pp) = self.tokenizer.get_padding_mut() {
            pp.strategy = tokenizers::PaddingStrategy::BatchLongest
        } else {
            let pp = PaddingParams {
                strategy: tokenizers::PaddingStrategy::BatchLongest,
                ..Default::default()
            };
            self.tokenizer.with_padding(Some(pp));
        }
        let tokens = self
            .tokenizer
            .encode_batch(inputs.to_vec(), true)
            .map_err(E::msg)?;
        let token_ids = tokens
            .iter()
            .map(|tokens| {
                let tokens = tokens.get_ids().to_vec();
                Ok(Tensor::new(tokens.as_slice(), device)?)
            })
            .collect::<Result<Vec<_>>>()?;

        let token_ids = Tensor::stack(&token_ids, 0)?;
        let token_type_ids = token_ids.zeros_like()?;
        let embeddings = self.model.forward(&token_ids, &token_type_ids)?;

        // Apply some avg-pooling by taking the mean embedding value for all tokens (including padding)
        let (_n_sentence, n_tokens, _hidden_size) = embeddings.dims3()?;
        let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;

        let mut results: Vec<Vec<f32>> = Vec::new();
        for j in 0..n_sentences {
            let e_j = embeddings.get(j)?;
            let mut emb: Vec<f32> = e_j.to_vec1()?;
            normalize(&mut emb);
            results.push(emb);
        }

        Ok(results)
    }
}

pub enum EmbeddingMsg {
    GetEmbedding {
        text: String,
        otx: tokio::sync::oneshot::Sender<Vec<f32>>,
    },
}

pub struct EmbeddingService {
    pub embedding_rx: Receiver<EmbeddingMsg>,
}

impl EmbeddingService {
    pub fn start(&mut self) {
        let mut embedding_provider = EmbeddingProvider::new().unwrap();

        while let Ok(message) = self.embedding_rx.recv() {
            match message {
                EmbeddingMsg::GetEmbedding { text, otx } => {
                    let start = Instant::now();
                    let mut r = embedding_provider
                        .calculate_embedding(&[text.as_str()])
                        .unwrap();
                    println!("[Embedding] Calculated in {:?}", start.elapsed());
                    otx.send(r.remove(0)).unwrap();
                }
            }
        }
    }
}
