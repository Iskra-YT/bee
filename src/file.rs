use std::fs::{self, ReadDir};

pub fn get_file_content(path: &String) -> String {
    return fs::read_to_string(path).unwrap();
}

pub fn get_directory(path: &String) -> ReadDir {
    return fs::read_dir(path).unwrap();
}