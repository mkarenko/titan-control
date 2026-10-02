//! Compiles the Blueprint UI (src/ui/window.blp) into GtkBuilder XML in OUT_DIR.
use std::{env, path::PathBuf, process::Command};

fn main() {
    let source = "src/ui/window.blp";
    println!("cargo:rerun-if-changed={source}");
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("window.ui");
    let status = Command::new("blueprint-compiler")
        .args(["compile", source, "--output"])
        .arg(&out)
        .status()
        .unwrap_or_else(|error| {
            panic!("blueprint-compiler is required to build the UI ({error}); install the `blueprint-compiler` package")
        });
    assert!(status.success(), "blueprint-compiler failed for {source}");
}
