use crate::fic::ensemble::Protection;
use crate::msc::decoder::{MainServiceChannelDecoder, new_decoder};
use crate::{fic::ensemble::Service, wavefinder::Buffer};
use bitvec::prelude::*;
use enum_dispatch::enum_dispatch;
use std::fmt;
use std::ops::Range;

mod cif;
mod decoder;
pub mod tables;

#[enum_dispatch]
trait BufferOps {
    fn reset(&mut self);
    fn push_buffer(&mut self, buffer: &MainServiceChannelBuffer) -> bool;
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
#[enum_dispatch(BufferOps)]
enum SizedBuffer {
    One(Buffers<1>),
    Two(Buffers<2>),
    Three(Buffers<3>),
}

pub struct Buffers<const N: usize> {
    sym: usize,
    lframe: usize,
    full: bool,
    pub symbols: [[Option<MainServiceChannelBuffer>; N]; 16],
}

#[derive(Debug)]
pub struct MainServiceChannel<'a> {
    service: &'a Service,
    symbols: ChannelSymbols,
    cur_frame: u8,
    cur_sym: u8,
    cifcnt: u64,
    buffers: SizedBuffer,
    decoder: MainServiceChannelDecoder,
}

impl<const N: usize> fmt::Debug for Buffers<{ N }> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        s.push_str(&format!("Buffers<{} ", N));
        for sym in self.symbols {
            for buf in sym {
                s.push_str(if buf.is_none() { " " } else { "X" });
            }
        }
        s.push('>');
        write!(f, "{}", s)
    }
}

impl<const N: usize> BufferOps for Buffers<{ N }> {
    fn reset(&mut self) {
        self.symbols = [[None; N]; 16];
        self.sym = 0;
        self.lframe = 0;
    }

    fn push_buffer(&mut self, buffer: &MainServiceChannelBuffer) -> bool {
        // dbg!("push buffer: lframe: {} sym: {}", self.lframe, self.sym);
        self.symbols[self.lframe][self.sym] = Some(*buffer);
        self.sym = (self.sym + 1) % N;
        if self.sym == 0 {
            self.lframe = (self.lframe + 1) % 16;
            if self.lframe == 0 {
                self.full = true;
            }
        }

        // complete frame?
        self.sym == 0 && self.full
    }
}

// one symbol, deinterleaved
#[derive(Clone, Copy)]
pub struct MainServiceChannelBuffer {
    bits: [u8; 3072],
}

impl fmt::Debug for MainServiceChannelBuffer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "MSCBuffer - {} non-zeros",
            self.bits.iter().filter(|a| **a != 0).count()
        )
    }
}

impl Default for MainServiceChannelBuffer {
    fn default() -> Self {
        Self { bits: [0; 3072] }
    }
}

#[derive(Debug)]
pub struct SymbolRange {
    start: u8,
    end: u8,
}

impl SymbolRange {
    pub fn length(&self) -> u8 {
        self.end - self.start
    }

    pub fn symbols(&self) -> Range<u8> {
        self.start..self.end + 1
    }
}

#[derive(Debug)]
pub struct ChannelSymbols {
    ranges: [SymbolRange; 4],
    startcu: u16,
    pub count: u16,
}

pub fn new_channel(service: &Service) -> MainServiceChannel<'_> {
    let decoder = new_decoder();
    let symbols = cif::channel_symbols(service);
    let buffers = match symbols.count {
        1 => SizedBuffer::One(Buffers::<1> {
            symbols: [[None; 1]; 16],
            sym: 0,
            lframe: 0,
            full: false,
        }),
        2 => SizedBuffer::Two(Buffers::<2> {
            symbols: [[None; 2]; 16],
            sym: 0,
            lframe: 0,
            full: false,
        }),
        3 => SizedBuffer::Three(Buffers::<3> {
            symbols: [[None; 3]; 16],
            sym: 0,
            lframe: 0,
            full: false,
        }),
        _ => panic!("unexpected count"),
    };
    MainServiceChannel {
        service,
        symbols,
        cur_frame: 0,
        cur_sym: 0,
        cifcnt: 0,
        buffers,
        decoder,
    }
}

#[derive(Debug)]
pub struct MainServiceChannelFrame {
    pub frame: u8,
    pub bitrate: u16,
    pub bits: Vec<u8>,
}

impl<'a> MainServiceChannel<'a> {
    pub fn try_buffer(&mut self, buffer: &Buffer) -> Option<MainServiceChannelFrame> {
        let symbol = buffer.bytes[2];
        let frame = buffer.bytes[3];

        if symbol <= 4 {
            return None;
        }
        if symbol == self.cur_sym {
            return None;
        }
        self.cur_sym = symbol;

        let mut buffer_full = false;

        // println!("subchsz: {} symbol: {} frame: {} cur_frame: {}", self.service.subchannel().size(), symbol, frame, self.cur_frame);

        if symbol == self.symbols.ranges[0].start {
            self.cur_frame = frame;
            buffer_full = self.buffers.push_buffer(&self.deinterleave(buffer));
        } else {
            for range in &self.symbols.ranges[1..4] {
                if symbol == range.start {
                    if frame == self.cur_frame {
                        buffer_full = self.buffers.push_buffer(&self.deinterleave(buffer));
                    } else {
                        dbg!("reset!");
                        self.buffers.reset();
                    }
                }
            }
            for range in &self.symbols.ranges {
                if symbol > range.start && symbol <= range.end {
                    if frame == self.cur_frame {
                        buffer_full = self.buffers.push_buffer(&self.deinterleave(buffer));
                        if symbol == range.end {
                            self.cifcnt += 1;
                        }
                    } else {
                        dbg!("reset!");
                        self.buffers.reset();
                    }
                }
            }
        }

        if buffer_full {
            Some(self.decode())
        } else {
            None
        }
    }

    fn decode(&self) -> MainServiceChannelFrame {
        let sc = self.service.subchannel();
        let bits = self
            .decoder
            .decode(&self.buffers, sc, &self.symbols);
        let bitrate = sc.bitrate();
        MainServiceChannelFrame {
            frame: self.cur_frame,
            bitrate,
            bits,
        }
    }

    fn deinterleave(&self, buffer: &Buffer) -> MainServiceChannelBuffer {
        self.decoder.deinterleave(buffer)
    }

    pub fn selstr(&self) -> [u8; 10] {
        let mut words: [u16; 5] = [0; 5];
        let bits = words.view_bits_mut::<Msb0>();

        for sr in &self.symbols.ranges {
            for sym in sr.symbols() {
                // avoid FIC symbols here
                if sym > 4 {
                    bits.set(sym as usize - 1, true);
                    bits.set(sym as usize - 2, true);
                }
            }
        }

        // always ask for FIC
        for bit in 0..3 {
            bits.set(bit, true);
        }

        // Safety: transmuting to a type with less strict alignment, u16 -> u8
        unsafe { std::mem::transmute::<[u16; 5], [u8; 10]>(words) }
    }
}
