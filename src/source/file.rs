use std::{
    fs::File,
    io::BufReader,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    thread::{self, JoinHandle},
};

use crate::{msc::MainServiceChannel, wavefinder::Buffer};

use super::Source;

pub struct FileSource {
    exit: Arc<Mutex<bool>>,
    path: Option<PathBuf>,
}

pub fn new_file_source(path: Option<PathBuf>) -> Box<dyn Source + Send + Sync> {
    let exit = Arc::new(Mutex::new(false));
    Box::new(FileSource { exit, path })
}

impl Source for FileSource {
    fn run(&mut self) -> (Receiver<Buffer>, JoinHandle<()>) {
        let (source_tx, source_rx) = mpsc::channel();
        let path = self.path.clone();
        let exit = self.exit.clone();
        let source_t = thread::spawn(move || {
            let mut buf;
            if let Some(p) = path {
                let file = File::open(&p);
                if let Ok(f) = file {
                    buf = BufReader::new(f);
                } else {
                    panic!("file couldn't be opened {:?}", p);
                }
            } else {
                panic!("no file specified");
            }

            loop {
                if let Ok(e) = exit.lock()
                    && *e
                {
                    break;
                }
                let result = Buffer::read_from_file(&mut buf);
                let Ok(buffer) = result else {
                    source_tx
                        .send(Buffer {
                            bytes: [0; 524],
                            last: true,
                        })
                        .unwrap();
                    break;
                };
                source_tx.send(buffer).unwrap();
            }
        });
        (source_rx, source_t)
    }

    fn select_channel(&mut self, channel: &MainServiceChannel) {
        dbg!(channel);
        // no-op for file source
    }

    fn ready(&self) -> bool {
        // file source is always ready
        true
    }

    fn exit(&mut self) {
        if let Ok(mut e) = self.exit.lock() {
            *e = true;
        }
    }
}
