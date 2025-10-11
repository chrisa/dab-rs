use crate::{fic::ensemble::Service, wavefinder::Buffer};
use bitvec::prelude::*;
use std::fmt;
use std::ops::Range;
use enum_dispatch::enum_dispatch;

mod cif;

#[enum_dispatch]
trait BufferOps {
    fn reset(&mut self);
    fn push_buffer(&mut self, buffer: &MainServiceChannelBuffer, i: usize, j: usize);
    fn frame_complete(&self) -> bool;
}

#[derive(Debug)]
#[enum_dispatch(BufferOps)]
enum SizedBuffer {
    One(Buffers<1>),
    Two(Buffers<2>),
    Three(Buffers<3>),
}

#[derive(Debug)]
pub struct MainServiceChannel<'a> {
    service: &'a Service,
    symbols: ChannelSymbols,
    cur_frame: u8,
    cifcnt: u64,
    buffers: SizedBuffer,
}

// all symbols for one frame
pub struct Buffers<const N: usize> {
    symbols: [[Option<MainServiceChannelBuffer>; N]; 4],
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
        s.push_str(">");
        write!(f, "{}", s)
    }
}

impl<const N: usize> BufferOps for Buffers<{N}> {

    fn reset(&mut self) {
        self.symbols = [[None; N]; 4];
    }

    fn push_buffer(&mut self, buffer: &MainServiceChannelBuffer, i: usize, j: usize) {
        self.symbols[i][j] = Some(*buffer);
    }

    fn frame_complete(&self) -> bool {
        for sym in self.symbols {
            for buf in sym {
                if buf.is_none() {
                    return false;
                }
            }
        }
        true
    }
}

// one symbol, deinterleaved
#[derive(Debug, Clone, Copy)]
pub struct MainServiceChannelBuffer {
    bytes: [u8; 3072],
}

impl Default for MainServiceChannelBuffer {
    fn default() -> Self {
        Self { bytes: [0; 3072] }
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
    endcu: u16,
    pub count: u16,
}

pub fn new_channel(service: &Service) -> MainServiceChannel<'_> {
    let symbols = cif::channel_symbols(service);
    let buffers = match symbols.count {
        1 => SizedBuffer::One(Buffers::<1> {
            symbols: [[None; 1]; 4],
        }),
        2 => SizedBuffer::Two(Buffers::<2> {
            symbols: [[None; 2]; 4],
        }),
        3 => SizedBuffer::Three(Buffers::<3> {
            symbols: [[None; 3]; 4],
        }),
        _ => panic!("unexpected count"),
    };
    MainServiceChannel {
        service,
        symbols,
        cur_frame: 0,
        cifcnt: 0,
        buffers,
    }
}

pub struct MainBlock {

}

impl<'a> MainServiceChannel<'a> {
    pub fn try_buffer(&mut self, buffer: &Buffer) -> Option<MainBlock> {
        let frame_symbol = buffer.bytes[2];
        let frame: u8 = buffer.bytes[3];

        if frame_symbol == self.symbols.ranges[0].start {
            self.cur_frame = frame;
            self.buffers.reset();
            self.buffers.push_buffer(&self.deinterleave(buffer), 0, 0);
        }

        for (i, range) in self.symbols.ranges.iter().enumerate() {
            for (j, symbol) in range.symbols().enumerate() {
                if frame_symbol == symbol && frame == self.cur_frame {
                    self.buffers.push_buffer(&self.deinterleave(buffer), i, j);
                }
            }
        }

        if self.buffers.frame_complete() {
            Some(self.decode())
        }
        else {
            None
        }
    }

    fn decode(&self) -> MainBlock {
        MainBlock {  } // run viterbi -> return data for PAD / MPEG
    }

    fn deinterleave(&self, _buffer: &Buffer) -> MainServiceChannelBuffer {
        MainServiceChannelBuffer::default() // run deinterleave
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
