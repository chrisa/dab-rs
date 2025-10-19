use std::cell::RefCell;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::msc::MainServiceChannel;
use crate::prs;
use crate::prs::sync::{PhaseReferenceSynchroniser, new_synchroniser};
use crate::wavefinder;
use crate::wavefinder::{Buffer, Wavefinder};

use super::Source;

static LOCKED: AtomicBool = AtomicBool::new(false);

pub struct WavefinderSource {
    tx: Sender<Buffer>,
    path: Option<PathBuf>,
    freq: String,
    sync: Option<Arc<Mutex<PhaseReferenceSynchroniser>>>,
}

pub fn new_wavefinder_source(
    tx: Sender<Buffer>,
    path: Option<PathBuf>,
    freq: Option<String>,
) -> impl Source {
    WavefinderSource {
        tx,
        path,
        freq: freq.unwrap_or("225.648".to_owned()),
        sync: None,
    }
}

impl Source for WavefinderSource {
    fn ready(&self) -> bool {
        if let Some(sync) = &self.sync
            && let Ok(s) = sync.lock()
        {
            return s.count() == 0;
        }
        false
    }

    fn select_channel(&mut self, channel: &MainServiceChannel) {
        // dbg!(channel);

        if let Some(sync) = &self.sync
            && let Ok(mut s) = sync.lock()
        {
            s.select_channel(channel);
        }
    }

    fn run(&mut self) -> JoinHandle<()> {
        let file_output = self.path.is_some();
        let path = self.path.clone();
        let tx = self.tx.clone();
        let freq = self.freq.clone();

        let sync = Arc::new(Mutex::new(new_synchroniser(&LOCKED)));
        self.sync = Some(sync.clone());

        thread::spawn(move || {
            let mut w: Wavefinder = wavefinder::open();
            let prs = RefCell::new(prs::new_symbol());

            let (message_tx, message_rx) = mpsc::channel();
            let (prs_tx, prs_rx) = mpsc::channel();
            let (file_tx, file_rx) = mpsc::channel::<Buffer>();

            thread::spawn(move || {
                loop {
                    let result = prs_rx.recv();
                    if let Ok(complete_prs) = result
                        && let Ok(mut s) = sync.lock()
                    {
                        let messages = s.try_sync_prs(complete_prs);
                        for m in messages {
                            message_tx.send(m).unwrap(); // handle Err?
                        }
                    }
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
                    tx.send(buffer).unwrap();

                    // File writer
                    if file_output {
                        file_tx.send(buffer).unwrap();
                    }
                }
            };

            w.set_callback(cb);

            if let Ok(f) = freq.parse::<f64>() {
                w.init(f); // BBC National DAB
            } else {
                panic!("bad frequency: {}", freq);
            }

            w.read();

            loop {
                w.handle_events();
                while let Ok(m) = message_rx.try_recv() {
                    w.send_ctrl_message(&m);
                }
            }
        })
    }
}
