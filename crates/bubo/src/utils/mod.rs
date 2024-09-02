use sha2::{Digest, Sha256};

pub mod time;
pub mod snowflake;
pub mod log;
pub mod prometheus;
pub mod validator;
pub mod error;
pub mod redis;
pub mod database;
pub mod serde;

pub fn sha256_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    let result_str = format!("{:x}", result);
    result_str
}