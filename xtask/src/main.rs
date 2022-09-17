mod build;
mod clean;
mod format;
mod lint;
mod spellcheck;
mod test;
mod utils;

use structopt::StructOpt;

use crate::{
    build::build, clean::clean, format::format, lint::lint, spellcheck::spellcheck, test::test,
};

#[derive(Debug, StructOpt)]
enum XTaskArgs {
    Build {
        #[structopt(long)]
        release: bool,
    },
    Clean,
    Format,
    Lint,
    Spellcheck,
    Test,
}

fn main() {
    let args = XTaskArgs::from_args();
    match args {
        XTaskArgs::Build { release } => build(release),
        XTaskArgs::Clean => clean(),
        XTaskArgs::Format => format(),
        XTaskArgs::Lint => lint(),
        XTaskArgs::Spellcheck => spellcheck(),
        XTaskArgs::Test => test(),
    }
}
