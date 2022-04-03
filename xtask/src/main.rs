mod build;
mod clean;
mod lint;
mod utils;

use structopt::StructOpt;

use crate::{build::build, clean::clean, lint::lint};

#[derive(Debug, StructOpt)]
enum XTaskArgs {
    Build {
        #[structopt(long)]
        release: bool,
    },
    Clean {},
    Lint {},
}

fn main() {
    let args = XTaskArgs::from_args();
    match args {
        XTaskArgs::Build { release } => build(release),
        XTaskArgs::Clean {} => clean(),
        XTaskArgs::Lint {} => lint(),
    }
}
