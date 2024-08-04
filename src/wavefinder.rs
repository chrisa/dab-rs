#![allow(dead_code)]

#[allow(clippy::all)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;

#[derive(Debug)]
pub struct Wavefinder {
    device: *mut wf_device,
    process: BufferCallback,
}

pub type BufferCallback = fn(*mut ::std::os::raw::c_uchar) -> ::std::os::raw::c_int;

pub fn open(rust_cb: BufferCallback) -> Wavefinder {
    let callback: process_func = Some({
        unsafe extern "C"
        fn cb (w: *mut wf_device, buf: *mut ::std::os::raw::c_uchar) -> ::std::os::raw::c_int
        {
            let callback = wf_callback(w) as *const();
            let callback: BufferCallback = unsafe { std::mem::transmute(callback) };
            callback(buf);
            0
        }
        cb
    });

    let w: &mut wf_device = unsafe { &mut *wf_open(callback, rust_cb as usize) };
    Wavefinder { device: w, process: rust_cb }
}

impl Drop for Wavefinder {
    fn drop(&mut self) {
        unsafe { wf_close(self.device) }
    }
}