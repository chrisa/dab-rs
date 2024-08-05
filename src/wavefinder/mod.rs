#![allow(dead_code)]

#[allow(clippy::all)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;

use std::{thread, time::Duration};

#[derive(Debug)]
pub struct Wavefinder {
    device: *mut wf_device,
}

#[derive(Debug)]
pub struct Buffer {
    pub bytes: [u8; 2048],
}

// Closure / callback implementation from:
// http://blog.sagetheprogrammer.com/neat-rust-tricks-passing-rust-closures-to-c

pub fn open<F>(buffer_callback: F) -> Wavefinder
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
    let w: &mut wf_device = unsafe { &mut *wf_open(Some(call_closure::<F>), data as *mut _) };
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
) where
    F: FnMut(Buffer),
{
    let callback_ptr = data as *mut F;
    let callback = &mut *callback_ptr;

    let slice = unsafe { std::slice::from_raw_parts(buf, 2048) };
    let buffer = Buffer {
        bytes: slice.try_into().unwrap(),
    };

    callback(buffer);
}

impl Drop for Wavefinder {
    fn drop(&mut self) {
        unsafe { wf_close(self.device) }
    }
}

mod init;
mod tune;

impl Wavefinder {
    pub fn read(&self) {
        unsafe {
            wf_read(self.device);
        }
    }

    fn mem_write(&self, addr: u16, val: u16) -> usize {
        let addr_bytes = addr.to_be_bytes();
        let val_bytes = val.to_be_bytes();

        let mut bytes = vec![addr_bytes[1], addr_bytes[0], val_bytes[1], val_bytes[0]];

        self.sendmem(addr as u32, val as u32, &mut bytes)
    }

    fn tune_msg(&self, reg: u32, bits: u8, pll: u8, lband: bool) -> usize {
        let reg_bytes = reg.to_be_bytes();
        let mut tbuf: [u8; 12] = [
            reg_bytes[3],
            reg_bytes[2],
            reg_bytes[1],
            reg_bytes[0],
            bits,
            0x00,
            pll,
            0x00,
            lband.into(),
            0x00,
            0x00,
            0x10,
        ];

        unsafe {
            let req: *mut wf_ctrl_request =
                wf_ctrl_request_init(WF_REQ_TUNE, 0, 0, tbuf.as_mut_ptr(), tbuf.len(), false);
            wf_usb_ctrl_msg(self.device, req)
        }
    }

    fn timing_msg(&self, buffer: &mut [u8; 32]) -> usize {
        unsafe {
            let req: *mut wf_ctrl_request =
                wf_ctrl_request_init(WF_REQ_TIMING, 0, 0, buffer.as_mut_ptr(), 32, false);
            wf_usb_ctrl_msg(self.device, req)
        }
    }

    fn sendmem(&self, value: u32, index: u32, buffer: &mut Vec<u8>) -> usize {
        unsafe {
            let req: *mut wf_ctrl_request = wf_ctrl_request_init(
                WF_REQ_SLMEM,
                value,
                index,
                buffer.as_mut_ptr(),
                buffer.len(),
                false,
            );
            wf_usb_ctrl_msg(self.device, req)
        }
    }

    fn r2_msg(&self, buffer: &mut [u8]) -> usize {
        unsafe {
            let req: *mut wf_ctrl_request =
                wf_ctrl_request_init(2, 0, 0x80, buffer.as_mut_ptr(), buffer.len(), false);
            wf_usb_ctrl_msg(self.device, req)
        }
    }

    fn r1_msg(&self, buffer: &mut [u8]) -> usize {
        unsafe {
            let req: *mut wf_ctrl_request =
                wf_ctrl_request_init(1, 0, 0x80, buffer.as_mut_ptr(), buffer.len(), false);
            wf_usb_ctrl_msg(self.device, req)
        }
    }

    fn sleep(&self, millis: u64) {
        thread::sleep(Duration::from_millis(millis));
    }
}
