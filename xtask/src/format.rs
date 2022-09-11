use std::process::Command;

use crate::utils::check_command_print_stdout;

pub(crate) fn format(check: bool) {
    format_rust(check);
    format_python();
}

fn format_rust(check: bool) {
    println!("Running cargo fmt");
    let mut fmt_cmd = Command::new("cargo");
    fmt_cmd.arg("fmt");
    if check {
        fmt_cmd.arg("--check");
    }
    check_command_print_stdout!(fmt_cmd, "Failed to run cargo fmt: {}");
}

fn format_python() {
    println!("Running flake8");
    let mut flake8_cmd = Command::new("flake8");
    flake8_cmd.args(&["--exclude", "ffi.py", "sdks/python/bowbend/"]);
    check_command_print_stdout!(flake8_cmd, "flake8 failed: {}");
}
