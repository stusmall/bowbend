use std::process::Command;

use crate::utils::{check_command, check_command_print_stdout, project_root};

pub(crate) fn test() {
    unit_test_rust();
    unit_test_python();
}

fn unit_test_rust() {
    println!("Running cargo test");
    let mut test_cmd = Command::new("cargo");
    test_cmd.current_dir(project_root());
    test_cmd.arg("test");
    check_command!(test_cmd, "Failed to run cargo test: {}");
}

fn unit_test_python() {
    println!("Running maturin develop");
    let mut maturin_cmd = Command::new("maturin");
    maturin_cmd.current_dir(project_root());
    maturin_cmd.args([
        "develop",
        "--extras",
        "lint,test",
        "-m",
        "sdks/python/Cargo.toml",
    ]);

    println!("Running pytest");

    let mut pytest_cmd = Command::new("pytest");
    pytest_cmd.current_dir(project_root());
    pytest_cmd.arg("sdks/python");
    check_command_print_stdout!(pytest_cmd, "Failed to run pytest: {}");
}
