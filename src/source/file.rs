use std::{io, path::PathBuf};

use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::wavefinder::Buffer;

pub struct FileControl {
    task: JoinHandle<()>,
}

impl FileControl {
    pub async fn shutdown(self) {
        self.task.abort();
        let _ = self.task.await;
    }
}

pub fn start_file_source(path: Option<PathBuf>) -> (mpsc::Receiver<Buffer>, FileControl) {
    let (source_tx, source_rx) = mpsc::channel(512);

    let task = tokio::task::spawn_local(async move {
        let Some(path) = path else {
            eprintln!("no file specified");
            let _ = source_tx
                .send(Buffer {
                    bytes: [0; 524],
                    last: true,
                })
                .await;
            return;
        };

        let mut file = match tokio::fs::File::open(&path).await {
            Ok(file) => file,
            Err(error) => {
                eprintln!("file couldn't be opened {:?}: {}", path, error);
                let _ = source_tx
                    .send(Buffer {
                        bytes: [0; 524],
                        last: true,
                    })
                    .await;
                return;
            }
        };

        loop {
            let mut bytes = [0; 524];
            match file.read_exact(&mut bytes).await {
                Ok(_) => {
                    if source_tx.send(Buffer { bytes, last: false }).await.is_err() {
                        break;
                    }
                }
                Err(error) => {
                    if error.kind() != io::ErrorKind::UnexpectedEof {
                        eprintln!("error reading {:?}: {}", path, error);
                    }
                    let _ = source_tx
                        .send(Buffer {
                            bytes: [0; 524],
                            last: true,
                        })
                        .await;
                    break;
                }
            }
        }
    });

    (source_rx, FileControl { task })
}
