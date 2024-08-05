#![allow(dead_code)]

#[allow(clippy::all)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;

use std::{thread, time::Duration};

use crate::prs::PhaseReferenceSymbol;

#[derive(Debug)]
pub struct Wavefinder {
    device: *mut wf_device,
    process: BufferCallback,
}

#[derive(Debug)]
pub struct Buffer {
    pub bytes: [u8; 2048],
}
pub type BufferCallback = fn(Buffer);
pub struct CallbackContext {
    pub prs: PhaseReferenceSymbol,
}

pub fn open(buffer_callback: BufferCallback) -> Wavefinder {
    let c_callback: process_func = Some({
        unsafe extern "C" fn cb(w: *mut wf_device, buf: *mut ::std::os::raw::c_uchar) {
            let tmp: BufferCallback = unsafe { std::mem::transmute(wf_callback(w) as *const ()) };
            // let ctx: &CallbackContext = unsafe { std::mem::transmute(wf_context(w) as *const ()) };
            let slice = unsafe { std::slice::from_raw_parts(buf, 2048) };
            let buffer = Buffer { bytes: slice.try_into().unwrap() };
            tmp(buffer);
        }
        cb
    });

    let w: &mut wf_device = unsafe { &mut *wf_open(c_callback, buffer_callback as usize) };
    Wavefinder {
        device: w,
        process: buffer_callback,
    }
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
            reg_bytes[0],
            reg_bytes[1],
            reg_bytes[2],
            reg_bytes[3],
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
