use std::{fs::create_dir_all, process::Command};

use crate::utils::{check_command, project_root};

pub(crate) fn build(release: bool) {
    build_rust(release);
    build_python(release);
}

fn build_rust(release: bool) {
    //TODO: Make colors auto detected so CI output doesn't look like shit
    println!("Building rust packages");
    let mut build_cmd = Command::new("cargo");
    build_cmd.arg("build").arg("--color=always");
    if release {
        build_cmd.arg("--release");
    }
    check_command!(build_cmd, "Failed to build rust project: {}");

    println!("Generating headers");
    let mut gen_header_cmd = Command::new("cargo");
    gen_header_cmd.args(&[
        "--color=always",
        "test",
        "--features",
        "c-headers",
        "--",
        "generate_headers",
    ]);
    check_command!(gen_header_cmd, "Failed to build headers: {}");
}

fn build_python(release: bool) {
    let mut manifest = project_root();
    manifest.push("sdks/python/Cargo.toml");

    create_dir_all("./sdks/python/target/")
        .expect("Failed to set up destination for python header file");
    println!("Prepping the header for cffi");
    let mut preprocessor_cmd = Command::new("gcc");
    preprocessor_cmd.args(&[
        "-E",
        &project_root()
            .join("target/portscanner_no_includes.h")
            .to_string_lossy(),
        "-o",
        &project_root()
            .join("sdks/python/target/header.h")
            .to_string_lossy(),
    ]);
    check_command!(preprocessor_cmd, "Running C preprocessor failed:  {}");

    println!("Building python wheels");
    let mut maturin_cmd = Command::new("maturin");
    if release {
        maturin_cmd.args(&["build", "--release", "-m", manifest.to_str().unwrap()]);
    } else {
        maturin_cmd.args(&["build", "-m", manifest.to_str().unwrap()]);
    }
    check_command!(maturin_cmd, "Python build failed:  {}");
}
