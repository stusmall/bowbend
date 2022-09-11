mod build;
mod clean;
mod format;
mod lint;
mod spellcheck;
mod utils;

use structopt::StructOpt;

use crate::{build::build, clean::clean, format::format, lint::lint, spellcheck::spellcheck};

#[derive(Debug, StructOpt)]
enum XTaskArgs {
    Build {
        #[structopt(long)]
        release: bool,
    },
    Clean,
    Format {
        #[structopt(long)]
        check: bool,
    },
    Lint,
    Spellcheck,
}

fn main() {
    let args = XTaskArgs::from_args();
    match args {
        XTaskArgs::Build { release } => build(release),
        XTaskArgs::Clean => clean(),
        XTaskArgs::Format { check } => format(check),
        XTaskArgs::Lint => lint(),
        XTaskArgs::Spellcheck => spellcheck(),
    }
}
