use std::process::Command;

use crate::utils::{check_command, check_command_print_stdout};

pub(crate) fn lint() {
    lint_rust();
    lint_python_sdk();
}

fn lint_rust() {
    //TODO: Run fmt on the excluded crate
    println!("Running clippy");
    let mut lint_cmd = Command::new("cargo");
    lint_cmd.args(&["clippy", "--color=always"]);
    check_command!(lint_cmd, "Failed to run cargo clippy: {}");
}

fn lint_python_sdk() {
    println!("Running pylint");
    let mut pylint_cmd = Command::new("pylint");
    pylint_cmd.args(&["--rcfile", "sdks/python/.pylintrc", "sdks/python/bowbend/"]);
    check_command_print_stdout!(pylint_cmd, "pylint failed: {}");

    println!("Running mypy");
    let mut flake8_cmd = Command::new("mypy");
    flake8_cmd.args(&["--config-file", "sdks/python/mypy.ini", "sdks/python/"]);
    check_command_print_stdout!(flake8_cmd, "mypy failed: {}");
    //TODO: spell check python
}
