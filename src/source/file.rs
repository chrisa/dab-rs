use std::{
    fs::File,
    io::BufReader,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc::Sender},
    thread::{self, JoinHandle},
};

use crate::{msc::MainServiceChannel, wavefinder::Buffer};

use super::Source;

pub struct FileSource {
    exit: Arc<Mutex<bool>>,
    tx: Sender<Buffer>,
    path: Option<PathBuf>,
}

pub fn new_file_source(tx: Sender<Buffer>, path: Option<PathBuf>) -> impl Source {
    let exit = Arc::new(Mutex::new(false));
    FileSource { exit, tx, path }
}

impl Source for FileSource {
    fn run(&mut self) -> JoinHandle<()> {
        let path = self.path.clone();
        let tx = self.tx.clone();
        let exit = self.exit.clone();
        thread::spawn(move || {
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
                    tx.send(Buffer {
                        bytes: [0; 524],
                        last: true,
                    })
                    .unwrap();
                    break;
                };
                tx.send(buffer).unwrap();
            }
        })
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
