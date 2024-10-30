#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::upper_case_acronyms)]

mod decode;
mod fic;
mod prs;
mod source;
mod wavefinder;

use std::{
    sync::mpsc::{self, Receiver},
    thread,
};

use clap::Parser;
use fic::{ensemble::new_ensemble, FastInformationChannelBuffer};
use source::Source;
use wavefinder::Buffer;

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
    let (tx, rx) = mpsc::channel();

    if args.source == CliSource::Wavefinder {
        go(
            rx,
            &source::wavefinder::new_wavefinder_source(tx, args.file),
        );
    } else if args.source == CliSource::File {
        go(rx, &source::file::new_file_source(tx, args.file));
    }
}

fn go(rx: Receiver<Buffer>, source: &impl Source) {
    let mut fic_decoder = fic::new_decoder();
    let mut ens = new_ensemble();

    let t = thread::spawn(move || {
        while let Ok(buffer) = rx.recv() {
            if buffer.last {
                break;
            }
            if let Ok(fic_buffer) = TryInto::<FastInformationChannelBuffer>::try_into(&buffer) {
                if let Some(fibs) = fic_decoder.try_buffer(fic_buffer) {
                    for fib in fibs {
                        let figs = fic_decoder.extract_figs(&fib);
                        for fig in figs {
                            ens.add_fig(fig);
                        }
                        if ens.is_complete() {
                            ens.display();
                        }
                    }
                }
            }
        }
    });

    source.run();
    t.join().unwrap();
}
