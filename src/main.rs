#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_many_arguments)]

use std::sync::mpsc::{self, Receiver};

use clap::Parser;
use dab::fic::{
    ensemble::{new_ensemble, Ensemble, Service},
    FastInformationChannelBuffer,
};
use dab::source::Source;
use dab::wavefinder::Buffer;

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

struct DABReceiver<'a> {
    rx: Receiver<Buffer>,
    source: &'a mut Box<dyn Source>,
    service_name: String,
}

fn main() {
    let args = Cli::parse();
    let (tx, rx) = mpsc::channel();

    let mut dab = match args.source {
        CliSource::Wavefinder => DABReceiver {
            source: &mut dab::source::wavefinder::new_wavefinder_source(tx, args.file),
            rx,
            service_name: args.service,
        },
        CliSource::File => DABReceiver {
            source: &mut dab::source::file::new_file_source(tx, args.file),
            rx,
            service_name: args.service,
        },
    };

    dab.go();
}

impl<'a> DABReceiver<'a> {
    fn go(&mut self) {
        let t = self.source.run();

        // FIC
        let ens = self.fic();
        ens.display();

        // If service, MSC
        if let Some(service) = ens.find_service(&self.service_name) {
            println!("Service '{}' found, playing", &self.service_name);
            self.source.as_mut().select_service(service);
            self.msc(service);
            t.join().unwrap();
        } else {
            println!("Service '{}' not found in ensemble", &self.service_name);
        }
    }

    fn fic(&self) -> Ensemble {
        let mut fic_decoder = dab::fic::new_decoder();
        let mut ens = new_ensemble();
        let service_name = self.service_name.to_owned();

        while let Ok(buffer) = self.rx.recv() {
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

    fn msc(&self, service: &Service) {
        while let Ok(buffer) = self.rx.recv() {
            if buffer.last {
                break;
            }
            // dbg!(buffer);
        }
    }
}
