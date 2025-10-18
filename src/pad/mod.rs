#![allow(non_snake_case)]

use std::io::{self, Write};

use bitvec::prelude::*;

use crate::msc::MainServiceChannelFrame;

#[derive(Debug, Clone)]
pub struct PadState {
    bitrate: i32,
    sampling_freq: i32,
    dls_length: u8,
    seglen: u8,
    left: u8,
    ci: u8,
    toggle: bool,
    first: u8,
    label: [u8; 128],
    crc: [u8; 2],
    segment: [u8; 20],
    ptr_index: usize,
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
        first: 0,
        label: [0; 128],
        crc: [0; 2],
        segment: [0; 20],
        ptr_index: 0,
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
    first: u8,
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
            first: bits[13..14].load_be(),
            toggle: bits[15],
        }
    }
}

impl PadState {
    pub fn ptr_mut(&mut self) -> &mut u8 {
        &mut self.segment[self.ptr_index]
    }

    pub fn output(&mut self, frame: &MainServiceChannelFrame) {
        let buf = &frame.bits;
        let bytes = buf.len();
        if bytes < 2 {
            return;
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
                    return;
                }

                let xpadoff = xpadoff as usize;

                if p.CIFlag {
                    self.ci = buf[xpadoff];
                    // eprintln!("bytes: {} xpadoff: {} self.ci: {}", bytes, xpadoff, self.ci);

                    if self.ci == 2 {
                        let prefix = u16::from_be_bytes([buf[xpadoff - 1], buf[xpadoff]]);
                        let dls = DlsPad::from_u16(prefix);

                        // dbg!(&dls);

                        if dls.first == 2 {
                            if dls.toggle != self.toggle {
                                self.toggle = dls.toggle;
                                let _ = writeln!(io::stderr(), "\nnew DLS:");
                            }
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
                        self.first = dls.first;
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
                        // crc16check(&self.segment, self.seglen);

                        if (self.seglen as usize) < self.segment.len() {
                            self.segment[self.seglen as usize] = 0;
                        }

                        eprintln!("segment: {}", String::from_utf8_lossy(&self.segment[2..]));

                        // let label_offset = (self.dls_length - (self.seglen - 2)) as usize;
                        // let copy_len = cmp::min((self.seglen - 2) as usize, self.label.len() - label_offset);
                        // self.label[label_offset..label_offset + copy_len]
                        //     .copy_from_slice(&self.segment[2..2 + copy_len]);

                        // if self.first == 1 {
                        //     self.label[self.dls_length as usize] = 0;
                        //     let text = String::from_utf8_lossy(&self.label[..self.dls_length as usize]);
                        //     let _ = writeln!(io::stderr(), "DLS: {}", text);
                        // }
                    }
                }
            }
        }
    }
}
