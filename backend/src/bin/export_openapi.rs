use std::{env, fs, path::PathBuf};

fn main() {
    let path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("openapi/knitprint.json"));
    let document = knitprint_api::openapi::document()
        .to_pretty_json()
        .expect("OpenAPI document should serialize");

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("OpenAPI output directory should be writable");
    }
    fs::write(&path, document).expect("OpenAPI document should be writable");
    println!("wrote {}", path.display());
}
