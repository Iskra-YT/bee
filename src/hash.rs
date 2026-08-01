use sha2::{Sha256, Digest};
use std::fs;
use std::io::{self, Read};

pub fn hash_string(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());

    let result = hasher.finalize();
    hex::encode(result)
}

pub fn hash_file(path: &str) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 { break; }
        hasher.update(&buffer[..count]);
    }

    Ok(hex::encode(hasher.finalize()))
}

pub fn bee_git_hash() -> &'static str {
    option_env!("BEE_GIT_HASH").unwrap_or("unknown")
}

pub fn write_hash_file(path: &str, value: &str) -> io::Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, value)?;
    fs::write(format!("{}.bee", path), bee_git_hash())?;
    Ok(())
}

