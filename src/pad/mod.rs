#![allow(non_snake_case)]

use crate::msc::MainServiceChannelFrame;
use bitvec::prelude::*;

pub struct Label {
    pub label: String,
    pub is_new: bool,
}

pub struct Error {}

const LABEL_MAX: usize = 128;
const SEGMENT_MAX: usize = 16;

#[derive(Debug, Clone)]
pub struct PadState {
    bitrate: i32,
    sampling_freq: i32,
    seglen: usize,
    ci: u8,
    toggle: bool,
    firstlast: FirstLast,
    label: [u8; LABEL_MAX],
    label_offset: usize,
    offset: usize,
    crc: [u8; 2],
    crc_offset: usize,
    segment: [u8; SEGMENT_MAX],
    segnum: u8,
    is_new: bool,
}

pub fn new_padstate(sampling_freq: i32) -> PadState {
    PadState {
        bitrate: 0,
        sampling_freq: sampling_freq,
        seglen: 0,
        ci: 0,
        toggle: false,
        firstlast: FirstLast::First,
        label: [0; LABEL_MAX],
        label_offset: 0,
        offset: 0,
        crc: [0; 2],
        crc_offset: 0,
        segment: [0; SEGMENT_MAX],
        segnum: 0,
        is_new: true, // assume first DLS label received is new
    }
}

/// Dummy placeholder for `crc16check`
fn _crc16check(_data: &[u8], _len: i32) {
    // In real code, implement CRC16 validation here
}

#[derive(Debug)]
struct FPad {
    _Z: bool,
    CIFlag: bool,
    _ByteL: u8,
    ByteL1: u8,
    FType: u8,
}

impl FPad {
    pub fn from_u16(bits: u16) -> Self {
        let bits = bits.view_bits::<Lsb0>();
        Self {
            _Z: bits[0],
            CIFlag: bits[1],
            _ByteL: bits[2..8].load_be(),
            ByteL1: bits[8..14].load_be(),
            FType: bits[14..16].load_be(),
        }
    }
}

#[derive(Debug)]
struct FPad00 {
    _ByteLInd: u8,
    XPadInd: XPadInd,
    _FType: u8,
}

impl FPad00 {
    pub fn from_u8(bits: u8) -> Self {
        let bits = bits.view_bits::<Lsb0>();
        Self {
            _ByteLInd: bits[0..4].load_be(),
            XPadInd: XPadInd::from_u8(bits[4..6].load_be()),
            _FType: bits[6..8].load_be(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum XPadInd {
    NoXPad = 0,
    ShortXPad = 1,
    VariableXPad = 2,
    _Reserved = 3,
}

impl XPadInd {
    pub fn from_u8(bits: u8) -> Self {
        match bits {
            0 => Self::NoXPad,
            1 => Self::ShortXPad,
            2 => Self::VariableXPad,
            u => panic!("unexpected XPadInd: {}", u),
        }
    }
}

#[derive(Debug)]
struct DlsPad {
    _rfa: u8,
    f2: u8,
    f1: u8,
    cmd: u8,
    firstlast: FirstLast,
    toggle: bool,
}

impl DlsPad {
    pub fn from_u16(bits: u16) -> Self {
        let bits = bits.view_bits::<Lsb0>();
        Self {
            _rfa: bits[0..4].load_be(),
            f2: bits[4..8].load_be(),
            f1: bits[8..12].load_be(),
            cmd: if bits[12] { 1 } else { 0 },
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
    pub fn output(&mut self, frame: &MainServiceChannelFrame) -> Result<Label, Error> {
        let bits = &frame.bits;
        let bytes = bits.len();
        if bytes < 2 {
            return Err(Error {});
        }

        let fpad = u16::from_be_bytes([bits[bytes - 2], bits[bytes - 1]]);
        let p = FPad::from_u16(fpad);

        if p.FType == 0 {
            let p00 = FPad00::from_u8(p.ByteL1);
            return self.fpad00(bits, p, p00);
        }

        Err(Error {})
    }

    fn scf_words(&self) -> usize {
        if self.sampling_freq == 48 {
            if self.bitrate >= 56 { 4 } else { 2 }
        } else {
            4
        }
    }

    fn fpad00(&mut self, bits: &[u8], p: FPad, p00: FPad00) -> Result<Label, Error> {
        if p00.XPadInd == XPadInd::ShortXPad {
            let xpadoff = bits.len() - (self.scf_words() + 2);
            let xpad = &bits[(xpadoff - 4)..xpadoff];

            if p.CIFlag {
                self.ci = xpad[3];
                if self.ci == 2 {
                    // DLS, start of X-PAD data group
                    let prefix = u16::from_be_bytes([xpad[2], xpad[1]]);
                    let dls = DlsPad::from_u16(prefix);

                    if dls.firstlast == FirstLast::First && dls.toggle != self.toggle {
                        self.toggle = dls.toggle;
                        self.is_new = true;
                    }

                    if dls.cmd == 0 {
                        // f1 is segment length
                        self.seglen = dls.f1 as usize + 1;
                        self.offset = 0;
                        self.crc_offset = 0;
                    }
                    if dls.cmd == 1 {
                        // f1 is "special command"
                        eprintln!("special command: {}", dls.f1);
                    }
                    if dls.firstlast == FirstLast::First || dls.firstlast == FirstLast::OneAndOnly {
                        eprintln!("charset: {}", dls.f2);
                    }
                    if dls.firstlast == FirstLast::Intermediate || dls.firstlast == FirstLast::Last
                    {
                        self.segnum = dls.f2;

                        // Catch first-time issue: coming in part way through a label
                        if self.segnum > 0 && self.label_offset == 0 {
                            self.ci = 0;
                            return Err(Error {});
                        }
                    }

                    self.segment = [0; 16];
                    self.segment[self.offset] = xpad[0];
                    self.offset += 1;
                    self.firstlast = dls.firstlast;

                    if self.segnum == 0 {
                        self.label = [0; LABEL_MAX];
                        self.label_offset = 0;
                    }
                }
            } else if self.ci == 2 {
                // previously set CI == 2
                for i in 0..4 {
                    if self.offset < self.seglen {
                        self.segment[self.offset] = xpad[3 - i];
                        self.offset += 1;
                    } else if self.crc_offset < 2 {
                        self.crc[self.crc_offset] = xpad[3 - i];
                        self.crc_offset += 1;
                    }
                }

                if self.offset == self.seglen && self.crc_offset == 2 {
                    self.label[self.label_offset..(self.label_offset + self.seglen)]
                        .copy_from_slice(&self.segment[0..self.seglen]);
                    self.label_offset += self.seglen;

                    if self.firstlast == FirstLast::Last {
                        let label_string = String::from_utf8_lossy(&self.label[0..self.label_offset]).to_string();
                        return Ok(Label {
                            is_new: self.is_new,
                            label: label_string,
                        });
                    }
                }
            }
        }
        Err(Error {})
    }
}
