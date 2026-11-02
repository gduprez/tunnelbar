use std::fs;
use std::path::Path;

fn main() {
    embed_locales();
    tauri_build::build()
}

/// Embeds every `locales/<lang>.json` so that adding a language needs no Rust change.
fn embed_locales() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../locales");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .expect("locales directory")
        .map(|entry| entry.expect("locale file").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort();
    let items: String = entries
        .iter()
        .map(|path| {
            let lang = path.file_stem().unwrap().to_string_lossy();
            let path = path.canonicalize().unwrap();
            format!(
                "    ({lang:?}, include_str!({:?})),\n",
                path.display().to_string()
            )
        })
        .collect();
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("locales.rs");
    fs::write(
        out,
        format!("pub const LOCALES: &[(&str, &str)] = &[\n{items}];\n"),
    )
    .unwrap();
}
