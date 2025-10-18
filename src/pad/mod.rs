#![allow(non_snake_case)]

use std::io::{self, Write};

use bitvec::prelude::*;

use crate::msc::MainServiceChannelFrame;


pub struct Label {
    pub label: String,
    pub is_new: bool,
}

pub struct Error {

}

#[derive(Debug, Clone)]
pub struct PadState {
    bitrate: i32,
    sampling_freq: i32,
    dls_length: u8,
    seglen: u8,
    left: u8,
    ci: u8,
    toggle: bool,
    firstlast: FirstLast,
    label: [u8; 128],
    offset: usize,
    crc: [u8; 2],
    segment: [u8; 18],
    ptr_index: usize,
    is_new: bool,
}

pub fn new_padstate() -> PadState {
    PadState {
        bitrate: 0,
        sampling_freq: 0,
        dls_length: 0,
        seglen: 0,
        left: 0,
        ci: 0,
        toggle: false,
        firstlast: FirstLast::First,
        label: [0; 128],
        offset: 0,
        crc: [0; 2],
        segment: [0; 18],
        ptr_index: 0,
        is_new: true, // assume first DLS label received is new
    }
}

/// Dummy placeholder for `crc16check`
fn crc16check(_data: &[u8], _len: i32) {
    // In real code, implement CRC16 validation here
}

#[derive(Debug)]
struct FPad {
    Z: bool,
    CIFlag: bool,
    ByteL: u8,
    ByteL1: u8,
    FType: u8,
}

impl FPad {
    pub fn from_u16(bits: u16) -> Self {
        let bits = bits.view_bits::<Lsb0>();
        Self {
            Z: bits[0],
            CIFlag: bits[1],
            ByteL: bits[2..8].load_be(),
            ByteL1: bits[8..14].load_be(),
            FType: bits[14..16].load_be(),
        }
    }
}

#[derive(Debug)]
struct FPad00 {
    ByteLInd: u8,
    XPadInd: u8,
    FType: u8,
}

impl FPad00 {
    pub fn from_u8(bits: u8) -> Self {
        let bits = bits.view_bits::<Lsb0>();
        Self {
            ByteLInd: bits[0..4].load_be(),
            XPadInd: bits[4..6].load_be(),
            FType: bits[6..8].load_be(),
        }
    }
}

#[derive(Debug)]
struct DlsPad {
    rfa: u8,
    f2: u8,
    f1: u8,
    cmd: bool,
    firstlast: FirstLast,
    toggle: bool,
}

impl DlsPad {
    pub fn from_u16(bits: u16) -> Self {
        let bits = bits.view_bits::<Lsb0>();
        Self {
            rfa: bits[0..4].load_be(),
            f2: bits[4..8].load_be(),
            f1: bits[8..12].load_be(),
            cmd: bits[12],
            firstlast: FirstLast::from_u8(bits[13..14].load_be()),
            toggle: bits[15],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum FirstLast {
    Intermediate = 0,
    Last = 1,
    First = 2,
    OneAndOnly = 3,
}

impl FirstLast {
    pub fn from_u8(bits: u8) -> Self {
        match bits {
            0 => Self::Intermediate,
            1 => Self::Last,
            2 => Self::First,
            3 => Self::OneAndOnly,
            u => panic!("unexpected FirstLast: {}", u),
        }
    }
}

impl PadState {
    pub fn ptr_mut(&mut self) -> &mut u8 {
        &mut self.segment[self.ptr_index]
    }

    pub fn output(&mut self, frame: &MainServiceChannelFrame) -> Result<Label, Error> {
        let buf = &frame.bits;
        let bytes = buf.len();
        if bytes < 2 {
            return Err(Error{});
        }

        let scf_words = if self.sampling_freq == 48 {
            if self.bitrate >= 56 { 4 } else { 2 }
        } else {
            4
        };

        let fpad = u16::from_be_bytes([buf[bytes - 2], buf[bytes - 1]]);

        // eprintln!("fpad: 0x{:x} bytes: {}", fpad, bytes);

        let p = FPad::from_u16(fpad);

        // dbg!(&p);

        if p.FType == 0 {
            let p00 = FPad00::from_u8(buf[bytes - 2]);

            // dbg!(&p00);

            if p00.XPadInd == 1 {
                let xpadoff = bytes as i32 - (1 + scf_words + 2);
                if xpadoff < 0 {
                    return Err(Error{})
                }

                let xpadoff = xpadoff as usize;

                if p.CIFlag {
                    self.ci = buf[xpadoff];
                    // eprintln!("bytes: {} xpadoff: {} self.ci: {}", bytes, xpadoff, self.ci);

                    if self.ci == 2 {
                        let prefix = u16::from_be_bytes([buf[xpadoff - 1], buf[xpadoff]]);
                        let dls = DlsPad::from_u16(prefix);

                        if dls.toggle != self.toggle {
                            self.toggle = dls.toggle;
                            self.is_new = true;
                        }

                        // dbg!(&dls);

                        if dls.firstlast == FirstLast::First {
                            self.ptr_index = 0;
                            self.dls_length = 0;
                        }

                        for i in 1..4 {
                            let idx = xpadoff.saturating_sub(i);
                            if idx < buf.len() && self.ptr_index < self.segment.len() {
                                self.segment[self.ptr_index] = buf[idx];
                                self.ptr_index += 1;
                            }
                        }

                        self.left = dls.f1 + 2;
                        self.seglen = self.left + 1;
                        self.dls_length += dls.f1 + 1;
                        self.firstlast = dls.firstlast;
                    }
                } else if self.ci == 2 {
                    for i in 0..4 {
                        if xpadoff < i {
                            break;
                        }
                        let idx = xpadoff - i;
                        if self.left > 2 {
                            self.segment[self.ptr_index] = buf[idx];
                            self.ptr_index += 1;
                            self.left -= 1;
                            if self.left == 2 {
                                self.ptr_index = 0; // simulate `ptr = crc`
                            }
                        } else if self.left > 0 {
                            self.crc[(2 - self.left) as usize] = buf[idx];
                            self.left -= 1;
                        }
                    }

                    if self.left == 0 {

                        // eprintln!("segment: {} seglen: {} firstlast: {:?}", String::from_utf8_lossy(&self.segment[2..]), self.seglen, self.firstlast);
                        match self.firstlast {
                            FirstLast::First => {
                                self.label = [0; 128];
                                self.offset = self.seglen as usize - 2;
                                self.label[0..self.offset].copy_from_slice(&self.segment[2..self.seglen as usize]);
                                return Err(Error{});
                            },
                            FirstLast::Intermediate => {
                                self.label[self.offset..(self.offset+self.seglen as usize - 2)].copy_from_slice(&self.segment[2..self.seglen as usize]);
                                self.offset += self.seglen as usize - 2;
                                return Err(Error{});
                            },
                            FirstLast::Last => {
                                self.label[self.offset..(self.offset+self.seglen as usize - 2)].copy_from_slice(&self.segment[2..self.seglen as usize]);
                                self.offset += self.seglen as usize - 2;
                                let label = Label { label: String::from_utf8_lossy(&self.label[0..self.offset]).to_string(), is_new: self.is_new };
                                // reset here, in case there is no First next time
                                self.label = [0; 128];
                                self.offset = 0;
                                self.is_new = false;
                                return Ok(label);
                            },
                            FirstLast::OneAndOnly => {
                                self.label = [0; 128];
                                self.offset = self.seglen as usize - 2;
                                self.label[0..self.offset].copy_from_slice(&self.segment[2..self.seglen as usize]);
                                let label = Label { label: String::from_utf8_lossy(&self.label[0..self.offset]).to_string(), is_new: self.is_new };
                                // reset here, in case there is no First next time
                                self.label = [0; 128];
                                self.offset = 0;
                                self.is_new = false;
                                return Ok(label);
                            }
                        }
                    }
                }
            }
        }
        Err(Error{})
    }
}
