use std::process::Command;

use crate::utils::{check_command, check_command_print_stdout, project_root};

pub(crate) fn lint() {
    lint_rust();
    lint_python_sdk();
}

fn lint_rust() {
    println!("Running clippy");
    let mut lint_cmd = Command::new("cargo");
    lint_cmd.current_dir(project_root());
    lint_cmd.args(["clippy", "--color=always", "--", "-Dwarnings"]);
    check_command!(lint_cmd, "Failed to run cargo clippy: {}");
}

fn lint_python_sdk() {
    println!("Running pylint");
    let mut pylint_cmd = Command::new("pylint");
    pylint_cmd.current_dir(project_root());
    pylint_cmd.args(["--rcfile", "sdks/python/.pylintrc", "sdks/python/bowbend/"]);
    check_command_print_stdout!(pylint_cmd, "pylint failed: {}");

    println!("Running mypy");
    let mut mypy_cmd = Command::new("mypy");
    mypy_cmd.current_dir(project_root());
    mypy_cmd.args(["--config-file", "sdks/python/mypy.ini", "sdks/python/"]);
    check_command_print_stdout!(mypy_cmd, "mypy failed: {}");
}
