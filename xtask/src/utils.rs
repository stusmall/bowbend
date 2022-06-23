use std::path::{Path, PathBuf};

macro_rules! check_command {
    ($cmd:tt, $msg:tt) => {
        let output = $cmd.output().unwrap();
        if !output.status.success() {
            let stderr = std::str::from_utf8(&output.stderr).unwrap();
            eprintln!($msg, format!("\n\n{}", stderr));
            eprintln!("Try again with command: {:?}", $cmd);
            std::process::exit(1);
        }
    };
}

macro_rules! check_command_print_stdout {
    ($cmd:tt, $msg:tt) => {
        let output = $cmd.output().unwrap();
        if !output.status.success() {
            let stderr = std::str::from_utf8(&output.stdout).unwrap();
            eprintln!($msg, format!("\n\n{}", stderr));
            eprintln!("Try again with command: {:?}", $cmd);
            std::process::exit(1);
        }
    };
}
pub(crate) use check_command;
pub(crate) use check_command_print_stdout;

pub(crate) fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
