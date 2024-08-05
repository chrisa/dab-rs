#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod wavefinder;
use prs::PhaseReferenceSymbol;
use wavefinder::{Buffer, Wavefinder};

mod prs;

fn main() {
    let mut prs: PhaseReferenceSymbol = prs::new();
    let cb = move |buffer: Buffer| {
        prs.try_buffer(buffer);
    };
    let w: Wavefinder = wavefinder::open(cb);
    w.init(225.648);
    w.read();
}
