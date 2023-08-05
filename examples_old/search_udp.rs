use std::io::{self, BufRead, Write};
use std::net::UdpSocket;
use std::time::Instant;

use dawnsearch::net::udp_messages::UdpMessage;
use rmp_serde::Serializer;
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};
use serde::Serialize;

use dawnsearch::vector::{Embedding, ToFrom24};

fn main() -> anyhow::Result<()> {
    let start = Instant::now();

    print!("Loading model...");

    let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
        .with_device(tch::Device::Cpu)
        .create_model()?;

    let duration = start.elapsed();
    println!(" {} ms", duration.as_millis());

    let stdin = io::stdin();
    println!("Ready. Please enter your search: ");
    println!("");
    print!("> ");
    io::stdout().flush()?;
    let mut previous_query = String::new();
    for q in stdin.lock().lines() {
        println!("");
        let mut query = q.unwrap();
        if query.is_empty() {
            query = previous_query.clone();
        } else {
            previous_query = query.clone();
        }

        let q = &model.encode(&[query]).unwrap()[0];
        let query_embedding: &Embedding<f32> = q.as_slice().try_into().unwrap();

        println!("Embedding {:?}", query_embedding);

        let socket = UdpSocket::bind("127.0.0.1:34254")?;

        let mut buf = Vec::new();
        let m = UdpMessage::Search {
            embedding: query_embedding.to24().as_slice().try_into().unwrap(),
        };
        println!("Sending {:?}", m);
        buf.clear();
        m.serialize(&mut Serializer::new(&mut buf)).unwrap();
        println!("");
        println!("Data: {} {:?}", buf.len(), buf);
        socket.send_to(&buf, "127.0.0.1:7231").unwrap();

        println!("");
        print!("> ");
        io::stdout().flush()?;
    }

    Ok(())
}
