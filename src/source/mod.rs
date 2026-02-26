use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::{CliSource, wavefinder::Buffer, wavefinder::Message};

pub mod file;
pub mod wavefinder;

pub struct SourceRuntime {
    pub buffers: mpsc::Receiver<Buffer>,
    pub control: SourceControl,
}

pub enum SourceControl {
    Wavefinder(wavefinder::WavefinderControl),
    File(file::FileControl),
}

impl SourceControl {
    pub fn send_wavefinder_message(&self, message: Message) {
        if let SourceControl::Wavefinder(control) = self {
            control.send_message(message);
        }
    }

    pub async fn shutdown(self) {
        match self {
            SourceControl::Wavefinder(control) => control.shutdown().await,
            SourceControl::File(control) => control.shutdown().await,
        }
    }
}

pub fn start_source(
    source: CliSource,
    file: Option<PathBuf>,
    frequency: Option<String>,
) -> SourceRuntime {
    match source {
        CliSource::Wavefinder => {
            let (buffers, control) = wavefinder::start_wavefinder_source(file, frequency);
            SourceRuntime {
                buffers,
                control: SourceControl::Wavefinder(control),
            }
        }
        CliSource::File => {
            let (buffers, control) = file::start_file_source(file);
            SourceRuntime {
                buffers,
                control: SourceControl::File(control),
            }
        }
    }
}
