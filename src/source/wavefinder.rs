use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

use tokio::sync::mpsc::Receiver;

use crate::wavefinder;
use crate::wavefinder::{Buffer, Message, Wavefinder};

enum WavefinderCommand {
    Stop,
    UsbMessage(Message),
}

pub struct WavefinderControl {
    command_tx: Sender<WavefinderCommand>,
    thread: Option<JoinHandle<()>>,
}

impl WavefinderControl {
    pub fn send_message(&self, message: Message) {
        let _ = self.command_tx.send(WavefinderCommand::UsbMessage(message));
    }

    pub async fn shutdown(mut self) {
        let _ = self.command_tx.send(WavefinderCommand::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = tokio::task::spawn_blocking(move || thread.join()).await;
        }
    }
}

pub fn start_wavefinder_source(
    path: Option<PathBuf>,
    frequency: Option<String>,
) -> (Receiver<Buffer>, WavefinderControl) {
    let (source_tx, source_rx) = tokio::sync::mpsc::channel(512);
    let (command_tx, command_rx) = mpsc::channel();

    let thread = thread::spawn(move || {
        let mut wavefinder: Wavefinder = wavefinder::open();
        let running = Arc::new(AtomicBool::new(true));
        let running_in_callback = running.clone();
        let source_tx_in_callback = source_tx;

        let mut file_output = path.and_then(|path| match File::create(path) {
            Ok(file) => Some(BufWriter::new(file)),
            Err(error) => {
                eprintln!("unable to create output file: {}", error);
                None
            }
        });

        let callback = move |buffer: Buffer| {
            if source_tx_in_callback.blocking_send(buffer).is_err() {
                running_in_callback.store(false, Ordering::Relaxed);
                return;
            }

            if let Some(file) = file_output.as_mut() {
                buffer.write_to_file(file);
            }
        };

        wavefinder.set_callback(callback);

        match frequency.as_deref().unwrap_or("225.648").parse::<f64>() {
            Ok(freq) => wavefinder.init(freq),
            Err(error) => {
                eprintln!("bad frequency {:?}: {}", frequency, error);
                running.store(false, Ordering::Relaxed);
            }
        }

        if running.load(Ordering::Relaxed) {
            wavefinder.read();
        }

        while running.load(Ordering::Relaxed) {
            while let Ok(command) = command_rx.try_recv() {
                match command {
                    WavefinderCommand::Stop => {
                        running.store(false, Ordering::Relaxed);
                    }
                    WavefinderCommand::UsbMessage(message) => {
                        wavefinder.send_ctrl_message(&message);
                    }
                }
            }

            if !running.load(Ordering::Relaxed) {
                break;
            }

            wavefinder.handle_events();
        }
    });

    (
        source_rx,
        WavefinderControl {
            command_tx,
            thread: Some(thread),
        },
    )
}
