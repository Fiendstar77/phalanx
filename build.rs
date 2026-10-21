//! Source identity for local replay compatibility; this is not an authentication signature.
use std::{fs, path::Path};

fn sources(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(dir).expect("read simulation sources") {
        let path = entry.expect("read source entry").path();
        if path.is_dir() {
            sources(&path, files);
        } else if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
}
fn main() {
    let mut files = Vec::new();
    sources(Path::new("src"), &mut files);
    files.extend(["build.rs", "Cargo.toml", "Cargo.lock", "assets/policy.json"].map(Into::into));
    files.sort();
    let mut hash = 0xcbf29ce484222325_u64;
    println!("cargo:rerun-if-changed=src");
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        for byte in path
            .to_string_lossy()
            .bytes()
            .chain([0])
            .chain(fs::read(&path).expect("read build input"))
        {
            hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
        }
    }
    println!("cargo:rustc-env=PHALANX_BUILD_ID={hash:016x}");
}
