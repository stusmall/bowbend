use std::process::Command;

use crate::utils::{check_command_print_stdout, project_root};

pub(crate) fn spellcheck() {
    spellcheck_rust()
}

fn spellcheck_rust() {
    println!("Running cargo spellcheck");
    let mut spell_check_cmd = Command::new("cargo");
    spell_check_cmd.current_dir(project_root());
    spell_check_cmd.args(["spellcheck", "--code", "1"]);
    check_command_print_stdout!(spell_check_cmd, "Failed to run cargo spellcheck: {}");
}
