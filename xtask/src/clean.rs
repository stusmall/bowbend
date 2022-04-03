use std::fs::remove_file;

use xshell::cmd;

pub(crate) fn clean() {
    let _ = remove_file("./sdks/python/target/header.h");
    cmd!("cargo clean")
        .read()
        .expect("Failed to run cargo clean");
    cmd!("cargo clean --manifest-path sdks/python/Cargo.toml")
        .read()
        .expect("Failed to run cargo clean");
}
