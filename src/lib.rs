use clap::Parser;

pub mod decode;
pub mod fic;
pub mod msc;
pub mod output;
pub mod pad;
pub mod prs;
pub mod source;
pub mod wavefinder;

pub mod receiver;

pub use decode::new_viterbi;

use crate::fic::ensemble::{Ensemble, Service};

pub enum EventData {
    Ensemble(Ensemble),
    Service(Service),
    Label(String),
}

pub struct UiEvent {
    pub data: EventData,
}

pub enum ControlData {
    Tune(f64),
    ServiceId(u16),
    Stop(),
}

pub struct ControlEvent {
    pub data: ControlData
}


#[derive(Debug, Clone, clap::ValueEnum, PartialEq)]
pub enum CliSource {
    Wavefinder,
    File,
}

#[derive(Parser, Debug)]
#[command(about, version)]
pub struct Cli {
    #[clap(value_enum, default_value_t=CliSource::Wavefinder)]
    source: CliSource,
    #[arg(short, long)]
    service: String,
    #[arg(short, long)]
    file: Option<std::path::PathBuf>,
    #[arg(long)]
    frequency: Option<String>,
}
