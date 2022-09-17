use std::process::Command;

use crate::utils::check_command_print_stdout;

pub(crate) fn spellcheck() {
    spellcheck_rust()
}

fn spellcheck_rust() {
    println!("Running cargo spellcheck");
    let mut lint_cmd = Command::new("cargo");
    lint_cmd.args(["spellcheck", "--code", "1"]);
    check_command_print_stdout!(lint_cmd, "Failed to run cargo spellcheck: {}");
}
