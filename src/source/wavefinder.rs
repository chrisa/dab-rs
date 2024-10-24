use std::cell::RefCell;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Sender};
use std::thread;

use crate::wavefinder::{Buffer, Wavefinder};
use crate::{prs, wavefinder};

use super::Source;

static LOCKED: AtomicBool = AtomicBool::new(false);

pub struct WavefinderSource {
    tx: Sender<Buffer>,
    path: Option<PathBuf>,
}

pub fn new_wavefinder_source(tx: Sender<Buffer>, path: Option<PathBuf>) -> impl Source {
    WavefinderSource { tx, path }
}

impl Source for WavefinderSource {
    fn run(&self) {
        let file_output = self.path.is_some();
        let mut w: Wavefinder = wavefinder::open();
        let prs = RefCell::new(prs::new_symbol());

        let (message_tx, message_rx) = mpsc::channel();
        let (prs_tx, prs_rx) = mpsc::channel();
        let (file_tx, file_rx) = mpsc::channel::<Buffer>();

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

        let path = self.path.clone();

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

        let tx = self.tx.clone();

        let cb = move |buffer: Buffer| {
            // Phase Reference Symbol
            prs.borrow_mut().try_buffer(&buffer);
            if prs.borrow_mut().is_complete() {
                let p = prs.replace_with(|_| prs::new_symbol());
                prs_tx.send(p).unwrap();
            }

            if LOCKED.load(std::sync::atomic::Ordering::Relaxed) {
                tx.send(buffer).unwrap();

                // File writer
                if file_output {
                    file_tx.send(buffer).unwrap();
                }
            }
        };

        w.set_callback(cb);

        w.init(225.648); // BBC National DAB

        // w.init(218.640); // Ayr

        // w.init(223.936); // D1 National (Scotland)
        // w.init(216.928); // Should be National 2
        // w.init(222.064); // Should be Central Scotland
        w.read();

        loop {
            w.handle_events();
            while let Ok(m) = message_rx.try_recv() {
                w.send_ctrl_message(&m);
            }
        }
    }
}
