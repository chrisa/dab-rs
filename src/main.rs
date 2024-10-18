#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod decode;
mod fic;
mod prs;
mod visualiser;
mod wavefinder;

use fic::FastInformationChannelBuffer;
use std::cell::RefCell;
use std::sync::mpsc;
use std::thread;
use wavefinder::{Buffer, Wavefinder};

fn main() {
    let mut w: Wavefinder = wavefinder::open();
    let prs = RefCell::new(prs::new_symbol());

    let (message_tx, message_rx) = mpsc::channel();
    let (prs_tx, prs_rx) = mpsc::channel();
    let (fic_tx, fic_rx) = mpsc::channel();

    thread::spawn(move || {
        let mut sync = prs::new_synchroniser();
        let result = prs_rx.recv();
        while let Ok(complete_prs) = result {
            let messages = sync.try_sync_prs(complete_prs);
            for m in messages {
                message_tx.send(m).unwrap(); // handle Err?
            }
        }
    });

    let cb = move |buffer: Buffer| {
        // Phase Reference Symbol
        prs.borrow_mut().try_buffer(&buffer);
        if prs.borrow_mut().is_complete() {
            let p = prs.replace_with(|_| prs::new_symbol());
            prs_tx.send(p).unwrap();
        }

        // Fast Information Channel
        if let Ok(fic_buffer) = TryInto::<FastInformationChannelBuffer>::try_into(&buffer) {
            fic_tx.send(fic_buffer).unwrap();
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
