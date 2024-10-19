#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod decode;
mod fic;
mod prs;
mod source;
mod wavefinder;

use clap::Parser;

#[derive(Debug, Clone, clap::ValueEnum, PartialEq)]
enum Source {
    Wavefinder,
    File,
}

#[derive(Parser, Debug)]
struct Cli {
    #[clap(value_enum, default_value_t=Source::Wavefinder)]
    source: Source,
    file: Option<std::path::PathBuf>,
}

fn main() {
    let args = Cli::parse();
    match args.source {
        Source::Wavefinder => source::wavefinder::run(args.file),
        Source::File => source::file::run(args.file),
    }
}
