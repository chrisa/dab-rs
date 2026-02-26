use std::thread::{self, JoinHandle};

use tokio::sync::mpsc;

use crate::msc::MainServiceChannelFrame;
use crate::output::mpeg;

const AUDIO_QUEUE_CAPACITY: usize = 128;

enum AudioCommand {
    Frame(MainServiceChannelFrame),
    Reset,
}

pub struct AudioWorker {
    tx: Option<mpsc::Sender<AudioCommand>>,
    thread: Option<JoinHandle<()>>,
}

impl Default for AudioWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioWorker {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel(AUDIO_QUEUE_CAPACITY);

        let thread = thread::spawn(move || {
            let mut mpeg = mpeg::new_mpeg();
            while let Some(command) = rx.blocking_recv() {
                match command {
                    AudioCommand::Frame(frame) => mpeg.output(&frame),
                    AudioCommand::Reset => mpeg.deinit(),
                }
            }
        });

        Self {
            tx: Some(tx),
            thread: Some(thread),
        }
    }

    pub fn try_send_frame(&self, frame: MainServiceChannelFrame) {
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(AudioCommand::Frame(frame));
        }
    }

    pub async fn reset(&self) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(AudioCommand::Reset).await;
        }
    }

    pub async fn shutdown(mut self) {
        drop(self.tx.take());
        if let Some(thread) = self.thread.take() {
            let _ = tokio::task::spawn_blocking(move || thread.join()).await;
        }
    }
}
