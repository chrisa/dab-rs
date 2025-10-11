use std::{
    fs::File,
    io::BufReader,
    path::PathBuf,
    sync::mpsc::Sender,
    thread::{self, JoinHandle},
};

use crate::{msc::cif::MainServiceChannel, wavefinder::Buffer};

use super::Source;

pub struct FileSource {
    tx: Sender<Buffer>,
    path: Option<PathBuf>,
}

pub fn new_file_source(tx: Sender<Buffer>, path: Option<PathBuf>) -> Box<dyn Source> {
    Box::new(FileSource { tx, path })
}

impl Source for FileSource {
    fn run(&self) -> JoinHandle<()> {
        let path = self.path.clone();
        let tx = self.tx.clone();
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
}
