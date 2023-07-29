use std::collections::HashMap;

pub const EM_LEN: usize = 384; // 300 for fasttext

pub fn find_embedding(
    embeddings: &HashMap<&str, &[f32; EM_LEN]>,
    s: &str,
    embedding: &mut [f32; 300],
) -> f32 {
    let mut embedding_scratch: [f32; EM_LEN] = [0.0; EM_LEN];
    let mut total = 0;
    let mut found = 0;
    for word in s.split(|c: char| !c.is_alphanumeric()) {
        if word.len() == 0 {
            continue;
        }
        total += 1;
        match embeddings.get(word) {
            Some(e) => {
                found += 1;
                for (i, v) in e.iter().enumerate() {
                    embedding_scratch[i] += *v;
                }
            }
            None => {}
        }
    }
    if found == 0 {
        embedding.fill(0.0); // Average
        return 0.0;
    }
    for (i, v) in embedding_scratch.iter().enumerate() {
        embedding[i] = v / found as f32;
    }
    found as f32 / total as f32
}

fn reduce_bits(x: f32) -> u8 {
    let mult = 16.0 * 16.0;
    let bits = ((x + 1.0) / 2.0 * mult) as u8;
    bits
}

pub fn distance(a: &[f32; EM_LEN], b: &[f32; EM_LEN]) -> f32 {
    let mut result: u32 = 0;
    for i in 0..EM_LEN {
        result += (reduce_bits(a[i]) as u32 - reduce_bits(b[i]) as u32).pow(2);
    }
    result as f32
}
