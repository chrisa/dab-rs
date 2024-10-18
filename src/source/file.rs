use std::{fs::File, io::{BufReader, Read}, path::PathBuf};

pub fn run(path: Option<PathBuf>) {
    let mut b;
    if let Some(p) = path {
        let file = File::open(&p);
        if let Ok(f) = file {
            b = BufReader::new(f);
        }
        else {
            panic!("file couldn't be opened {:?}", p);
        }
    }
    else {
        panic!("no file specified");
    }

    loop {
        let mut buffer: [u8; 524] = [0; 524];
        let result = b.read_exact(&mut buffer);
        if let Err(r) = result {
            panic!("read error: {:?}", r);
        }
        dbg!(buffer);
    }
}