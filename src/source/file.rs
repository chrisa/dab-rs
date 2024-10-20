use std::{fs::File, io::BufReader, path::PathBuf, sync::mpsc::Sender};

use crate::{
    fic::FastInformationChannelBuffer,
    wavefinder::Buffer,
};

use super::Source;

pub struct FileSource {
    fic_tx: Sender<FastInformationChannelBuffer>,
    path: Option<PathBuf>,
}

pub fn new_file_source(fic_tx: Sender<FastInformationChannelBuffer>, path: Option<PathBuf>) -> impl Source {
    FileSource {
        fic_tx,
        path
    }
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
            let buffer = Buffer::read_from_file(&mut buf);
    
            // Fast Information Channel
            if let Ok(fic_buffer) = TryInto::<FastInformationChannelBuffer>::try_into(&buffer) {
                self.fic_tx.send(fic_buffer).unwrap();
            }
        }
    }    
}

