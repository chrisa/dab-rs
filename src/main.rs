#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod wavefinder;
use wavefinder::Wavefinder;

fn cb(buf: *mut ::std::os::raw::c_uchar) -> ::std::os::raw::c_int {
    println!("in rust cb: {:?}", buf);
    0
}

fn main() {
    let w: Wavefinder = wavefinder::open(cb);
    w.init(225.648);
    w.read();
}
