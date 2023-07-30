use std::collections::HashMap;

use rand::Rng;

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

/**
 * The range of i8 is -128 to 127.
 */
fn reduce_bits(x: f32) -> i16 {
    let mult = 16.0 * 16.0 / 2.0;
    let mut bits = (x * mult) as i32;
    if bits == 128 {
        bits = 127;
    }
    bits as i16
}

pub fn distance(a: &[f32; EM_LEN], b: &[f32; EM_LEN]) -> f32 {
    let mut result: f32 = 0.0;
    for i in 0..EM_LEN {
        result += (a[i] - b[i]).powf(2.0);
    }
    result as f32
}

pub fn distance_upper_bound(a: &[f32; EM_LEN], b: &[f32; EM_LEN], limit: f32) -> f32 {
    let mut result: f32 = 0.0;
    for i in 0..EM_LEN {
        result += (a[i] - b[i]).powf(2.0);
        if result > limit {
            return limit;
        }
    }
    result as f32
}

pub fn distance_reduced(a: &[f32; EM_LEN], b: &[f32; EM_LEN]) -> f32 {
    let mut result: u32 = 0;
    for i in 0..EM_LEN {
        result += (reduce_bits(a[i]) as i32 - reduce_bits(b[i]) as i32).pow(2) as u32;
    }
    result as f32
}

pub fn distance_i8(a: &[i8; EM_LEN], b: &[i8; EM_LEN]) -> u32 {
    let mut result: u32 = 0;
    for i in 0..EM_LEN {
        result += (a[i] as i32 - b[i] as i32).pow(2) as u32;
    }
    result
}

/**
 * Random unit length vector.
 */
pub fn random_address() -> [f32; EM_LEN] {
    let mut rng = rand::thread_rng();
    let mut address: [f32; EM_LEN] = [0.0; EM_LEN];
    for x in 0..EM_LEN {
        address[x] = rng.gen();
    }
    let length = vector_length(&address);
    for x in 0..EM_LEN {
        address[x] /= length;
    }
    address
}

pub fn vector_length(v: &[f32; EM_LEN]) -> f32 {
    distance(v, &[0.0; EM_LEN]).sqrt()
}
