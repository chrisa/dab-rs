use crate::{fic::ensemble::Service, wavefinder::Buffer};
use bitvec::prelude::*;
use std::fmt;
use std::ops::Range;
use enum_dispatch::enum_dispatch;

mod cif;

#[enum_dispatch]
trait BufferOps {
    fn reset(&mut self);
    fn push_buffer(&mut self, buffer: &Buffer, i: usize, j: usize);
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
    symbols: [[MainServiceChannelBuffer; N]; 4],
}

impl<const N: usize> fmt::Debug for Buffers<{ N }> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Buffers<{}>", N)
    }
}

impl<const N: usize> BufferOps for Buffers<{N}> {

    fn reset(&mut self) {

    }

    fn push_buffer(&mut self, buffer: &Buffer, i: usize, j: usize) {

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
            symbols: [[MainServiceChannelBuffer::default(); 1]; 4],
        }),
        2 => SizedBuffer::Two(Buffers::<2> {
            symbols: [[MainServiceChannelBuffer::default(); 2]; 4],
        }),
        3 => SizedBuffer::Three(Buffers::<3> {
            symbols: [[MainServiceChannelBuffer::default(); 3]; 4],
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

impl<'a> MainServiceChannel<'a> {
    pub fn try_buffer(&mut self, buffer: &Buffer) {
        let symbol = buffer.bytes[2];
        let frame: u8 = buffer.bytes[3];

        // println!("symbol: {} frame: {}", symbol, frame);

        if symbol == self.symbols.ranges[0].start {
            self.cur_frame = frame;
            self.buffers.reset();
            self.buffers.push_buffer(buffer, 0, 0);
        }
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
