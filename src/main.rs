#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod wavefinder;
use wavefinder::{Buffer, CallbackContext, Wavefinder};

mod prs;

fn cb(buffer: Buffer) {
    println!("in rust cb: {:?}", buffer);
    // let mut prs = ctx.prs;
    // prs.try_buffer(buffer);
}

fn main() {
    let prs = prs::new();

    let ctx = CallbackContext { prs: prs };
    let w: Wavefinder = wavefinder::open(cb);
    w.init(225.648);
    w.read();
}
