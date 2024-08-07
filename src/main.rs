#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod prs;
mod visualiser;
mod wavefinder;

use wavefinder::{Buffer, Wavefinder};

fn main() {
    let mut w: Wavefinder = wavefinder::open();
    let mut sync = prs::new_synchroniser();
    let mut prs = prs::new_symbol();

    let cb = move |buffer: Buffer| {
        prs.try_buffer(buffer);
        if prs.is_complete() {
            // let i = ifft(prs.vector());
            // vis.update(i);
            let (c, ir) = sync.try_sync_prs(&prs);
            dbg!(c, ir);
            prs = prs::new_symbol();
        }
    };
    w.set_callback(cb);

    // let (prs_syms, prs_conj) = prs::prs_reference();
    // let prs_syms_ifft = ifft(prs_syms);
    // let prs_conj_ifft = ifft(prs_conj);
    // let vis1_scale = 128.0;
    // let mut vis1 = visualiser::create_visualiser("PRS reference", 400, 400, -vis1_scale..vis1_scale, -vis1_scale..vis1_scale);
    // vis1.update(prs_conj_ifft);

    w.init(225.648);
    w.read();
}
