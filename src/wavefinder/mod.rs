#![allow(dead_code)]

#[allow(clippy::all)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;
pub use message::*;

use std::{thread, time::Duration};

#[derive(Debug)]
pub struct Wavefinder {
    device: *mut wf_device,
}

#[derive(Debug)]
pub struct Buffer {
    pub bytes: [u8; 524],
}

// Closure / callback implementation from:
// http://blog.sagetheprogrammer.com/neat-rust-tricks-passing-rust-closures-to-c

pub fn open() -> Wavefinder {
    let w: &mut wf_device = unsafe { &mut *wf_open() };
    Wavefinder { device: w }
}

// Safety: The pointer passed to this function must be
// a valid non-null pointer of type `F`. We've carefully
// reviewed the documentation for our C lib and know
// that is the case.
unsafe extern "C" fn call_closure<F>(
    _w: *mut wf_device,
    data: *mut ::std::os::raw::c_void,
    buf: *mut ::std::os::raw::c_uchar,
    len: usize,
) where
    F: FnMut(Buffer),
{
    let callback_ptr = data as *mut F;
    let callback = &mut *callback_ptr;
    let slice = unsafe { std::slice::from_raw_parts(buf, len) };
    if let Ok(bytes) = slice.try_into() {
        callback(Buffer { bytes });
    } else {
        println!("short read? len = {:?}", len);
    }
}

impl Drop for Wavefinder {
    fn drop(&mut self) {
        unsafe { wf_close(self.device) }
    }
}

mod init;
mod message;
mod tune;

impl Wavefinder {
    pub fn set_callback<F>(&mut self, buffer_callback: F)
    where
        F: FnMut(Buffer) + 'static,
    {
        let data = Box::into_raw(Box::new(buffer_callback));

        // Safety: We've carefully reviewed the docs for the C function
        // we're calling, and the variants we need to uphold are:
        // - widget is a valid pointer
        //    - We're using Rust references so we know this is true.
        // - data is valid until its destructor is called
        //     - We've added a `'static` bound to ensure that is true.
        unsafe { wf_set_callback(self.device, Some(call_closure::<F>), data as *mut _) };
    }

    pub fn read(&self) {
        unsafe {
            wf_read(self.device);
        }
    }

    pub fn handle_events(&self) {
        unsafe {
            wf_handle_events();
        }
    }

    pub fn send_ctrl_message(&self, message: &Message) -> usize {
        let ptr = Box::into_raw(message.bytes.clone()) as *mut u8;
        unsafe {
            let req: *mut wf_ctrl_request = wf_ctrl_request_init(
                code_for_kind(&message.kind),
                message.value,
                message.index,
                ptr,
                message.size,
                message.async_,
            );
            let result = wf_usb_ctrl_msg(self.device, req);
            let _bytes = Box::from_raw(ptr);
            result
        }
    }

    fn sendmem(&self, value: u32, index: u32, buffer: &Vec<u8>) -> usize {
        let message = message::slmem_msg(value, index, buffer);
        self.send_ctrl_message(&message)
    }

    fn mem_write(&self, addr: u16, val: u16) -> usize {
        let message = message::mem_write_msg(addr, val);
        self.send_ctrl_message(&message)
    }

    fn tune_msg(&self, reg: u32, bits: u8, pll: u8, lband: bool) -> usize {
        let message = message::tune_msg(reg, bits, pll, lband);
        self.send_ctrl_message(&message)
    }

    fn timing_msg(&self, buffer: &mut [u8; 32]) -> usize {
        let message = message::timing_msg(buffer);
        self.send_ctrl_message(&message)
    }

    fn r2_msg(&self) -> usize {
        let message = message::r2_msg();
        self.send_ctrl_message(&message)
    }

    fn r1_msg(&self) -> usize {
        let message = message::r1_msg();
        self.send_ctrl_message(&message)
    }

    fn sleep(&self, millis: u64) {
        thread::sleep(Duration::from_millis(millis));
    }
}
