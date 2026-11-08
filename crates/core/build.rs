use std::{env, fs, path::PathBuf};

fn main() {
    let root =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../assets/submodels");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files: Vec<_> = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    let mut code = String::from("pub fn get(name: &str) -> Option<&'static [u8]> { match name {\n");
    for file in files {
        let name = file.file_name().unwrap().to_str().unwrap();
        code.push_str(&format!(
            "{name:?} => Some(include_bytes!({:?})),\n",
            file.canonicalize().unwrap()
        ));
    }
    code.push_str("_ => None, } }\n");
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("bundled_resources.rs"),
        code,
    )
    .unwrap();
}
