use std::fs::remove_file;

use xshell::{cmd, Shell};

pub(crate) fn clean() {
    let sh = Shell::new().unwrap();
    let _ = remove_file("./sdks/python/target/header.h");
    cmd!(sh, "cargo clean")
        .read()
        .expect("Failed to run cargo clean");
    cmd!(sh, "cargo clean --manifest-path sdks/python/Cargo.toml")
        .read()
        .expect("Failed to run cargo clean");
}
