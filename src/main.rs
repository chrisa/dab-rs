#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_many_arguments)]

mod decode;
mod fic;
mod msc;
mod prs;
mod source;
mod wavefinder;

use std::sync::mpsc::{self, Receiver};

use clap::Parser;
use fic::{
    ensemble::{new_ensemble, Ensemble, Service},
    FastInformationChannelBuffer,
};
use msc::cif::channel_symbols;
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
    #[arg(short, long)]
    service: String,
    #[arg(short, long)]
    file: Option<std::path::PathBuf>,
}

fn main() {
    let args = Cli::parse();
    let (tx, rx) = mpsc::channel();

    if args.source == CliSource::Wavefinder {
        go(
            rx,
            &source::wavefinder::new_wavefinder_source(tx, args.file),
            &args.service,
        );
    } else if args.source == CliSource::File {
        go(
            rx,
            &source::file::new_file_source(tx, args.file),
            &args.service,
        );
    }
}

fn go(rx: Receiver<Buffer>, source: &impl Source, service_name: &str) {
    let t = source.run();

    // FIC
    let ens = fic(&rx, service_name);
    ens.display();

    // If service, MSC
    if let Some(service) = ens.find_service(service_name) {
        println!("Service '{}' found, playing", &service_name);
        msc(&rx, service);
        t.join().unwrap();
    }
    else {
        println!("Service '{}' not found in ensemble", &service_name);
    }
}

fn fic(rx: &Receiver<Buffer>, service_name: &str) -> Ensemble {
    let mut fic_decoder = fic::new_decoder();
    let mut ens = new_ensemble();
    let service_name = service_name.to_owned();

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
                }
                if ens.is_complete() {
                    break;
                }
            }
        }
    }
    ens
}

fn msc(rx: &Receiver<Buffer>, service: &Service) {
    dbg!(channel_symbols(service));
    while let Ok(buffer) = rx.recv() {
        if buffer.last {
            break;
        }
        // dbg!(buffer);
    }
}
