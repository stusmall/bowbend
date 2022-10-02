use std::{fs::create_dir_all, process::Command};

use crate::utils::{check_command, project_root};

pub(crate) fn build(release: bool, asan: bool) {
    build_rust(release, asan);
    build_python(release, asan);
}

fn build_rust(release: bool, asan: bool) {
    println!("Building rust packages");
    let mut build_cmd = Command::new("cargo");
    build_cmd.current_dir(project_root());
    build_cmd.arg("build").arg("--color=always");
    if release {
        build_cmd.arg("--release");
    }
    if asan {
        build_cmd.env("RUSTFLAGS", "-Z sanitizer=address");
        build_cmd.arg("--target");
        build_cmd.arg("x86_64-unknown-linux-gnu");
    }
    check_command!(build_cmd, "Failed to build rust project: {}");

    println!("Generating headers");
    let mut gen_header_cmd = Command::new("cargo");
    gen_header_cmd.current_dir(project_root());
    gen_header_cmd.args([
        "--color=always",
        "test",
        "--package=ffi",
        "--features",
        "c-headers",
        "--",
        "generate_headers",
    ]);
    check_command!(gen_header_cmd, "Failed to build headers: {}");
}

fn build_python(release: bool, asan: bool) {
    create_dir_all("./sdks/python/target/")
        .expect("Failed to set up destination for python header file");
    println!("Prepping the header for cffi");
    let mut preprocessor_cmd = Command::new("gcc");
    preprocessor_cmd.current_dir(project_root());
    preprocessor_cmd.args([
        "-E",
        &project_root()
            .join("target/bowbend_no_includes.h")
            .to_string_lossy(),
        "-o",
        &project_root()
            .join("sdks/python/target/header.h")
            .to_string_lossy(),
    ]);
    check_command!(preprocessor_cmd, "Running C preprocessor failed:  {}");

    println!("Building python wheels");
    let mut maturin_cmd = Command::new("maturin");
    maturin_cmd.current_dir(project_root());
    maturin_cmd.arg("build");
    if release {
        maturin_cmd.arg("--release");
    }
    if asan {
        maturin_cmd.arg("--rustc-extra-args=-Clink-arg=-lasan -Zsanitizer=address");
        maturin_cmd.arg("--target");
        maturin_cmd.arg("x86_64-unknown-linux-gnu");
    }
    maturin_cmd.arg("-m");
    maturin_cmd.arg("sdks/python/Cargo.toml");
    check_command!(maturin_cmd, "Python build failed:  {}");
}
