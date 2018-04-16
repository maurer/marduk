use std::fs;

fn main() {
    for entry_r in fs::read_dir("mycroft").unwrap() {
        let entry = entry_r.unwrap();
        if let Some(ext) = entry.path().extension() {
            if ext == "my" {
                println!("cargo:rerun-if-changed={}", entry.path().display())
            }
        }
    }
}
