use std::fs::remove_dir_all;

use xshell::{cmd, Shell};

use crate::utils::project_root;

pub(crate) fn clean() {
    let sh = Shell::new().unwrap();
    let mut vagrant_folder = project_root();
    vagrant_folder.push(".vagrant");
    let _ = remove_dir_all(vagrant_folder);
    cmd!(sh, "cargo clean")
        .read()
        .expect("Failed to run cargo clean");
    cmd!(sh, "cargo clean --manifest-path sdks/python/Cargo.toml")
        .read()
        .expect("Failed to run cargo clean");
}
