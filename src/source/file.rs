use std::{fs::File, io::BufReader, path::PathBuf, sync::mpsc::Sender};

use crate::wavefinder::Buffer;

use super::Source;

pub struct FileSource {
    tx: Sender<Buffer>,
    path: Option<PathBuf>,
}

pub fn new_file_source(tx: Sender<Buffer>, path: Option<PathBuf>) -> impl Source {
    FileSource { tx, path }
}

impl Source for FileSource {
    fn run(&self) {
        let mut buf;
        let path = self.path.clone();
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
                self.tx
                    .send(Buffer {
                        bytes: [0; 524],
                        last: true,
                    })
                    .unwrap();
                break;
            };
            self.tx.send(buffer).unwrap();
        }
    }
}
