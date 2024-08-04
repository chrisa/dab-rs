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

mod init;
mod tune;

impl Wavefinder {

    pub fn read(&self)
    {
        unsafe {
            wf_read(self.device);
        }
    }

    fn mem_write(&self, addr: u32, val: u16) -> usize
    {
        let addr_bytes = addr.to_be_bytes();
        let val_bytes = val.to_be_bytes();

    	let mut bytes = vec!(
            addr_bytes[3],
            addr_bytes[2],
            val_bytes[1],
            val_bytes[0],
        );
        
        self.sendmem(addr as u32, val as u32, &mut bytes)
    }

    fn tune_msg(&self, reg: u32, bits: u8, pll: u8, lband: bool) -> usize
    {
        let reg_bytes = reg.to_be_bytes();
        let mut tbuf: [u8; 12] = [
            reg_bytes[0], 
            reg_bytes[1], 
            reg_bytes[2], 
            reg_bytes[3], 
            bits,
            0x00,
            pll.into(),
            0x00,
            lband.into(),
            0x00,
            0x00,
            0x10,
        ];

        unsafe {
            let req: *mut wf_ctrl_request = wf_ctrl_request_init(WF_REQ_TUNE, 0, 0, tbuf.as_mut_ptr(), tbuf.len(), false);
            wf_usb_ctrl_msg(self.device, req)
        }
    }

    fn timing_msg(&self, buffer: &mut [u8; 32]) -> usize
    {
        unsafe {
            let req: *mut wf_ctrl_request = wf_ctrl_request_init(WF_REQ_TIMING, 0, 0, buffer.as_mut_ptr(), 32, false);
            wf_usb_ctrl_msg(self.device, req)
        }
    }

    fn sendmem(&self, value: u32, index: u32, buffer: &mut Vec<u8>) -> usize
    {
        // println!("{:?}", buffer);
        // for b in buffer.clone() {
        //     print!("{:#04x} ", b);
        // }
        // println!("");

        unsafe {
            let req: *mut wf_ctrl_request = wf_ctrl_request_init(WF_REQ_SLMEM, value, index, buffer.as_mut_ptr(), buffer.len(), false);
            wf_usb_ctrl_msg(self.device, req)
        }
    }

    fn r2_msg(&self, buffer: &mut[u8]) -> usize
    {
	    // usb_ctrl_msg(wf, 2, 0, 0x80, bytes, 64))
        unsafe {
            let req: *mut wf_ctrl_request = wf_ctrl_request_init(2, 0, 0x80, buffer.as_mut_ptr(), buffer.len(), false);
            println!("{:?}", req);
            wf_usb_ctrl_msg(self.device, req)
        }
    }

    fn r1_msg(&self, buffer: &mut[u8]) -> usize
    {
	    // return(wf_usb_ctrl_msg(wf, 1, 0, 0x80, bytes, 64));
        unsafe {
            let req: *mut wf_ctrl_request = wf_ctrl_request_init(1, 0, 0x80, buffer.as_mut_ptr(), buffer.len(), false);
            println!("{:?}", req);
            wf_usb_ctrl_msg(self.device, req)
        }
    }

    fn sleep(&self, millis: u64)
    {
        thread::sleep(Duration::from_millis(millis));
    }

}