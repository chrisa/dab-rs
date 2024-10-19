use std::{fs::File, io::BufReader, path::PathBuf};

use crate::{fic::{self, FastInformationChannelBuffer}, wavefinder::Buffer};

pub fn run(path: Option<PathBuf>) {
    let mut buf;
    if let Some(p) = path {
        let file = File::open(&p);
        if let Ok(f) = file {
            buf = BufReader::new(f);
        }
        else {
            panic!("file couldn't be opened {:?}", p);
        }
    }
    else {
        panic!("no file specified");
    }

    let mut fic = fic::new_decoder();

    loop {
        let buffer = Buffer::read_from_file(&mut buf);

        // Fast Information Channel
        if let Ok(fic_buffer) = TryInto::<FastInformationChannelBuffer>::try_into(&buffer) {
            fic.try_buffer(fic_buffer);
        }
    }
}