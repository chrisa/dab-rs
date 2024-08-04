#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[allow(clippy::all)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;

fn main() {

    let callback: process_func = Some({
        unsafe extern "C"
        fn cb (w: *mut wavefinder, buf: *mut ::std::os::raw::c_uchar) -> ::std::os::raw::c_int
        {
            println!("{:?}", w);
            println!("{:?}", buf);
            0
        }
        cb
    });


    // let callback = Some(cb);
    let w: &mut wavefinder = unsafe { &mut *wf_open(callback) };
    println!("{:?}", w);
    unsafe { wf_close(w) };
}