use fic::FastInformationChannelBuffer;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::io::BufWriter;
use std::fs::File;

use crate::{fic, wavefinder, prs};
use crate::wavefinder::{Buffer, Wavefinder};

static LOCKED: AtomicBool = AtomicBool::new(false);

pub fn run(path: Option<PathBuf>) {
    let file_output = path.is_some();
    let mut w: Wavefinder = wavefinder::open();
    let prs = RefCell::new(prs::new_symbol());

    let (message_tx, message_rx) = mpsc::channel();
    let (prs_tx, prs_rx) = mpsc::channel();
    let (file_tx, file_rx) = mpsc::channel::<Buffer>();
    let (fic_tx, fic_rx) = mpsc::channel();

    thread::spawn(move || {
        let mut sync = prs::new_synchroniser(&LOCKED);
        loop {
            let result = prs_rx.recv();
            if let Ok(complete_prs) = result {
                let messages = sync.try_sync_prs(complete_prs);
                for m in messages {
                    message_tx.send(m).unwrap(); // handle Err?
                }
            }
        }
    });

    thread::spawn(move || {
        let mut fic = fic::new_decoder();
        while let Ok(buffer) = fic_rx.recv() {
            fic.try_buffer(buffer);
        }
    });

    if file_output {
        thread::spawn(move || {
            if let Some(p) = path {
                let f = File::create(p).expect("Unable to create file");
                let mut buf = BufWriter::new(f);

                loop {
                    let result = file_rx.recv();
                    if let Ok(buffer) = result {
                        buffer.write_to_file(&mut buf);
                    }
                }
            }
        });

    }

    let cb = move |buffer: Buffer| {
        // Phase Reference Symbol
        prs.borrow_mut().try_buffer(&buffer);
        if prs.borrow_mut().is_complete() {
            let p = prs.replace_with(|_| prs::new_symbol());
            prs_tx.send(p).unwrap();
        }

        if LOCKED.load(std::sync::atomic::Ordering::Relaxed) {
            // Fast Information Channel
            if let Ok(fic_buffer) = TryInto::<FastInformationChannelBuffer>::try_into(&buffer) {
                fic_tx.send(fic_buffer).unwrap();
            }

            // File writer
            if file_output {
                file_tx.send(buffer).unwrap();
            }
        }
    };

    w.set_callback(cb);
    w.init(225.648);
    w.read();

    loop {
        w.handle_events();
        while let Ok(m) = message_rx.try_recv() {
            w.send_ctrl_message(&m);
        }
    }
}