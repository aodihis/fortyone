use std::fs;
use std::path::Path;

fn main() {
    let env_path = Path::new(".env");
    if let Ok(contents) = fs::read_to_string(env_path) {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if !key.is_empty() {
                    println!("cargo:rustc-env={}={}", key, value);
                }
            }
        }
    }
    println!("cargo:rerun-if-changed=.env");
}
