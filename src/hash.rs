use sha2::{Sha256, Digest};

pub fn hash_string(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());

    let result = hasher.finalize();
    hex::encode(result)
}

pub fn hash_string_with_salt(input: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher.update(salt.as_bytes());

    let result = hasher.finalize();
    hex::encode(result)
}