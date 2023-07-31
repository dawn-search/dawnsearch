use std::iter::zip;

use num::Num;
use rand::Rng;

pub const EM_LEN: usize = 384; // 300 for fasttext

pub type Embedding<T> = [T; EM_LEN];

fn f32_to_i16(x: f32) -> i16 {
    (x * i16::MAX as f32).round() as i16
}

pub trait ToI16 {
    fn to_i16(&self) -> Embedding<i16>;
}

impl ToI16 for Embedding<f32> {
    fn to_i16(&self) -> Embedding<i16> {
        let mut result: [i16; EM_LEN] = [0; EM_LEN];
        for i in 0..EM_LEN {
            result[i] = f32_to_i16(self[i]);
        }
        result
    }
}

pub trait Distance<T: SupportedNum, Y> {
    fn distance(&self, other: &Embedding<T>) -> Y;
    fn distance_ip(&self, other: &Embedding<T>) -> Y;
}

impl Distance<f32, f32> for Embedding<f32> {
    fn distance(&self, b: &Embedding<f32>) -> f32 {
        zip(self, b).map(|(a, b)| (a - b).powf(2.0)).sum()
    }

    fn distance_ip(&self, b: &Embedding<f32>) -> f32 {
        zip(self, b).map(|(a, b)| a * b).sum()
    }
}

impl Distance<i16, u64> for Embedding<i16> {
    fn distance(&self, b: &Embedding<i16>) -> u64 {
        zip(self, b)
            .map(|(a, b)| (*a as i64 - *b as i64).pow(2))
            .sum::<i64>() as u64
    }
    fn distance_ip(&self, b: &Embedding<i16>) -> u64 {
        (i64::MAX
            - zip(self, b)
                .map(|(a, b)| *a as i64 * *b as i64)
                .sum::<i64>()) as u64
    }
}

pub trait SupportedNum: Num + PartialOrd + Copy {}

impl SupportedNum for i16 {}
impl SupportedNum for f32 {}

/**
 * Cosine distance.
 *
 * Note: much slower, and doesn't seem to have quality benefits.
 */
pub fn distance_cosine(a: &Embedding<f32>, b: &Embedding<f32>) -> f32 {
    let mut result: f32 = 0.0;
    for i in 0..EM_LEN {
        result += a[i] * b[i]
    }
    1.0 - result
}

pub fn distance_upper_bound(a: &Embedding<f32>, b: &Embedding<f32>, _limit: f32) -> f32 {
    let mut result: f32 = 0.0;
    for i in 0..EM_LEN {
        result += (a[i] - b[i]).powf(2.0);
        // Float additions are not vectorized anyway.
        // This is < 10% faster, so not really worth it.
        // if result > limit {
        //     return f32::INFINITY;
        // }
    }
    result as f32
}

pub fn distance_reduced(a: &Embedding<f32>, b: &Embedding<f32>) -> f32 {
    let mut result: u32 = 0;
    for i in 0..EM_LEN {
        result += (f32_to_i16(a[i]) as i32 - f32_to_i16(b[i]) as i32).pow(2) as u32;
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
pub fn random_address() -> Embedding<f32> {
    let mut rng = rand::thread_rng();
    let mut address: Embedding<f32> = [0.0; EM_LEN];
    for x in 0..EM_LEN {
        address[x] = rng.gen();
    }
    let length = vector_length(&address);
    for x in 0..EM_LEN {
        address[x] /= length;
    }
    address
}

pub fn vector_length(v: &Embedding<f32>) -> f32 {
    v.distance(&[0.0; EM_LEN]).sqrt()
}
