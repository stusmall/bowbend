use std::{path::PathBuf, process::Command};

use crate::utils::{check_command_print_stdout, project_root};

pub(crate) fn spellcheck() {
    spellcheck_rust()
}

fn spellcheck_rust() {
    println!("Running cargo spellcheck");
    let home = std::env::var("HOME").unwrap();
    let mut path = PathBuf::from(home);
    path.push(".cargo/bin/cargo-spellcheck");

    let mut spell_check_cmd = Command::new(path);
    spell_check_cmd.current_dir(project_root());
    // This is pretty gross.  I'm going this so it will work in CI with a prebuilt a
    // binary and locally when installed with cargo install.
    spell_check_cmd.args(["--code", "1"]);
    check_command_print_stdout!(spell_check_cmd, "Failed to run cargo spellcheck: {}");
}
