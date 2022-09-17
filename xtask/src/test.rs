use std::process::Command;

use crate::utils::{check_command, check_command_print_stdout, project_root};

pub(crate) fn test() {
    unit_test_rust();
    unit_test_python();
}

fn unit_test_rust() {
    println!("Running cargo test");
    let mut test_cmd = Command::new("cargo");
    test_cmd.arg("test");
    check_command!(test_cmd, "Failed to run cargo test: {}");
}

fn unit_test_python() {
    println!("Running maturin develop");
    let mut manifest = project_root();
    manifest.push("sdks/python/Cargo.toml");
    let mut maturin_cmd = Command::new("maturin");
    maturin_cmd.args([
        "develop",
        "--extras",
        "lint,test",
        "-m",
        manifest.to_str().unwrap(),
    ]);

    println!("Running pytest");
    let mut python_base = project_root();
    python_base.push("sdks/python");

    let mut pytest_cmd = Command::new("pytest");
    pytest_cmd.arg(python_base);
    check_command_print_stdout!(pytest_cmd, "Failed to run pytest: {}");
}
