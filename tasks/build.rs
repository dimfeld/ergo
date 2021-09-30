use std::{fs, path};

fn build_snapshot(dir: &path::Path, name: &str, extensions: Vec<ergo_js::Extension>) {
    let mut runtime = ergo_js::Runtime::new(ergo_js::RuntimeOptions {
        will_snapshot: true,
        extensions,
        ..Default::default()
    });

    let snapshot = runtime.make_snapshot();
    let output = dir.join(name);
    fs::write(&output, &snapshot).unwrap();
}

fn main() {
    println!("cargo:rerun-if-changed=../js");
    println!("cargo:rerun-if-changed=build.rs");
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripting")
        .join("snapshots");
    let e = std::fs::DirBuilder::new().create(&dir);
    if let Err(e) = e {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Error creating directory: {}", e);
        }
    }

    build_snapshot(&dir, "core", ergo_js::core_extensions(None));
    build_snapshot(&dir, "net", ergo_js::net_extensions(None));
}
