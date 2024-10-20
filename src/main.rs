#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod decode;
mod fic;
mod prs;
mod source;
mod wavefinder;

use std::{sync::mpsc::{self, Receiver}, thread};

use clap::Parser;
use fic::FastInformationChannelBuffer;
use source::Source;

#[derive(Debug, Clone, clap::ValueEnum, PartialEq)]
enum CliSource {
    Wavefinder,
    File,
}

#[derive(Parser, Debug)]
struct Cli {
    #[clap(value_enum, default_value_t=CliSource::Wavefinder)]
    source: CliSource,
    file: Option<std::path::PathBuf>,
}

fn main() {
    let args = Cli::parse();
    let (fic_tx, fic_rx) = mpsc::channel();

    if args.source == CliSource::Wavefinder {
        go(fic_rx, &source::wavefinder::new_wavefinder_source(fic_tx, args.file));
    }
    else if args.source == CliSource::File {
        go(fic_rx, &source::file::new_file_source(fic_tx, args.file));
    }
}

fn go(fic_rx: Receiver<FastInformationChannelBuffer>, source: &impl Source) {

    thread::spawn(move || {
        let mut fic = fic::new_decoder();
        while let Ok(buffer) = fic_rx.recv() {
            fic.try_buffer(buffer);
        }
    });

    source.run();
}
