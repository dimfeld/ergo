use glob::glob;
use std::{
    fs,
    path::{Path, PathBuf},
};

fn build_snapshot(dir: &Path, name: &str, extensions: Vec<ergo_js::Extension>) {
    let runtime = ergo_js::Runtime::new(ergo_js::RuntimeOptions {
        will_snapshot: true,
        extensions,
        ..Default::default()
    });

    let snapshot = runtime.make_snapshot();
    let output = dir.join(name);
    fs::write(&output, &snapshot).unwrap();
}

fn build_snapshots() {
    println!("cargo:rerun-if-changed=../js");
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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

fn build_js_helpers() {
    println!("cargo:rerun-if-changed=js_helpers/src");
    println!("cargo:rerun-if-changed=js_helpers/rollup.config.js");
    println!("cargo:rerun-if-changed=scripting/js_helpers");

    let input_glob = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripting")
        .join("js_helpers")
        .join("*.js");
    let mut files = glob(input_glob.to_string_lossy().as_ref())
        .expect("finding js_helper files")
        .collect::<Result<Vec<_>, _>>()
        .expect("finding js_helper files");
    files.sort();

    let output = files
        .into_iter()
        .map(|path| {
            fs::read_to_string(&path).map(|script| {
                format!(
                    "// {}\n{}",
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    script
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .expect("Reading files")
        .join("\n\n");

    let output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripting")
        .join("task_helpers.js");
    fs::write(&output_path, output).expect("writing task_helpers.js");
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    build_snapshots();
    build_js_helpers();
}
