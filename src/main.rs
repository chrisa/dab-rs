#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_many_arguments)]

use std::sync::mpsc::{self, Receiver};

use clap::Parser;
use dab::source::Source;
use dab::wavefinder::Buffer;
use dab::{
    fic::{
        FastInformationChannelBuffer,
        ensemble::{Ensemble, new_ensemble},
    },
    msc::{MainServiceChannel, new_channel},
};

#[derive(Debug, Clone, clap::ValueEnum, PartialEq)]
enum CliSource {
    Wavefinder,
    File,
}

#[derive(Parser, Debug)]
#[command(about, version)]
struct Cli {
    #[clap(value_enum, default_value_t=CliSource::Wavefinder)]
    source: CliSource,
    #[arg(short, long)]
    service: String,
    #[arg(short, long)]
    file: Option<std::path::PathBuf>,
    #[arg(long)]
    frequency: Option<String>,
}

struct DABReceiver<'a> {
    rx: Receiver<Buffer>,
    source: &'a mut Box<dyn Source>,
    service_id: String,
}

fn main() {
    let args = Cli::parse();
    let (tx, rx) = mpsc::channel();

    let mut dab = match args.source {
        CliSource::Wavefinder => DABReceiver {
            source: &mut dab::source::wavefinder::new_wavefinder_source(
                tx,
                args.file,
                args.frequency,
            ),
            rx,
            service_id: args.service,
        },
        CliSource::File => DABReceiver {
            source: &mut dab::source::file::new_file_source(tx, args.file),
            rx,
            service_id: args.service,
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
        if let Some(service) = ens.find_service_by_id(&self.service_id) {
            println!("Service ID '{:04x}' found, {}", service.id, service.name);
            let mut msc = new_channel(service);
            self.source.as_mut().select_channel(&msc);
            self.msc(&mut msc);
            t.join().unwrap();
        } else {
            println!("Service '{}' not found in ensemble", &self.service_id);
        }
    }

    fn fic(&self) -> Ensemble {
        let mut fic_decoder = dab::fic::new_decoder();
        let mut ens = new_ensemble();
        let service_name = self.service_id.to_owned();

        while let Ok(buffer) = self.rx.recv() {
            if buffer.last {
                break;
            }
            if let Ok(fic_buffer) = TryInto::<FastInformationChannelBuffer>::try_into(&buffer)
                && let Some(fibs) = fic_decoder.try_buffer(fic_buffer)
            {
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
        ens
    }

    fn msc(&self, channel: &mut MainServiceChannel) {
        while let Ok(buffer) = self.rx.recv() {
            if buffer.last {
                break;
            }

            if let Some(main) = channel.try_buffer(&buffer) {
                // dbg!(&main);
            }
        }
    }
}
