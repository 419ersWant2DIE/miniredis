#![allow(dead_code)]
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

pub fn rand_i32() -> i32 {
    let mut rng = thread_rng();
    let num : i32 = rng.gen();
    num.abs() % 10 + 1
}

pub fn rand_str() -> String {
    let len = rand_i32();
    let str = thread_rng()
    .sample_iter(&Alphanumeric)
    .take(len as usize)
    .map(char::from)
    .collect();
    if str == "Null" {
        "shabi".to_string()
    }else {
        str
    }
}

fn main() {}