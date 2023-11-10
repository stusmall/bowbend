use xshell::{cmd, Shell};

use crate::utils::project_root;

pub(crate) fn clean() {
    let sh = Shell::new().unwrap();
    sh.change_dir(project_root());
    cmd!(sh, "cargo clean")
        .read()
        .expect("Failed to run cargo clean");
    cmd!(sh, "cargo clean --manifest-path sdks/python/Cargo.toml")
        .read()
        .expect("Failed to run cargo clean");
    cmd!(sh, "cargo clean --manifest-path sdks/rust/Cargo.toml")
        .read()
        .expect("Failed to run cargo clean");
    cmd!(
        sh,
        "cargo clean --manifest-path integration/rust/Cargo.toml"
    )
    .read()
    .expect("Failed to run cargo clean");
}
