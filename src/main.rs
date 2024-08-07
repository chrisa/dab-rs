#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod prs;
mod visualiser;
mod wavefinder;

use wavefinder::{Buffer, Channel, Reader, Wavefinder, Writer};

fn main() {
    let channel = Box::new(Channel::new());
    let s_channel: &'static mut Channel = Box::leak(channel);
    let writer = Writer::new(s_channel);
    let reader = Reader::new(s_channel);

    let mut w: Wavefinder = wavefinder::open();
    let mut sync = prs::new_synchroniser();
    let mut prs = prs::new_symbol();

    let cb = move |buffer: Buffer| {
        prs.try_buffer(buffer);
        if prs.is_complete() {
            let messages = sync.try_sync_prs(&prs);
            for m in messages {
                writer.write(m);
            }
            prs = prs::new_symbol();
        }
    };

    w.set_callback(cb);
    w.init(225.648);
    w.read();

    loop {
        w.handle_events();
        while let Some(m) = reader.read() {
            w.send_ctrl_message(&m);
        }
    }
}
