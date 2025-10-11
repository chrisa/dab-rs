use crate::{fic::ensemble::Service, wavefinder::Buffer};
use bitvec::prelude::*;
use std::fmt;
use std::ops::Range;

#[derive(Debug)]
enum SizedBuffer {
    One(Box<Buffers<1>>),
    Two(Box<Buffers<2>>),
    Three(Box<Buffers<3>>),
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
    let symbols = channel_symbols(service);
    let buffers = match symbols.count {
        1 => SizedBuffer::One(Box::new(Buffers::<1> {
            symbols: [[MainServiceChannelBuffer::default(); 1]; 4],
        })),
        2 => SizedBuffer::Two(Box::new(Buffers::<2> {
            symbols: [[MainServiceChannelBuffer::default(); 2]; 4],
        })),
        3 => SizedBuffer::Three(Box::new(Buffers::<3> {
            symbols: [[MainServiceChannelBuffer::default(); 3]; 4],
        })),
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

const MSCSTART: u16 = 5;
const CUSPERSYM: u16 = 48;
const SYMSPERCIF: u8 = 18;

pub fn channel_symbols(service: &Service) -> ChannelSymbols {
    let subchannel = service.subchannel();

    let size = subchannel.size();
    let start = subchannel.startaddr();
    let startcu = start % CUSPERSYM;
    let endcu = (start + size) % CUSPERSYM;

    let symbol_0 = SymbolRange {
        start: (start / CUSPERSYM + MSCSTART) as u8,
        end: ((start + size) / CUSPERSYM + MSCSTART) as u8,
    };
    let count = symbol_0.length() as u16 + 1;

    let symbols: Vec<SymbolRange> = (0..4)
        .scan(symbol_0, |state, _i| {
            let this = SymbolRange {
                start: state.start,
                end: state.end,
            };
            state.start += SYMSPERCIF;
            state.end += SYMSPERCIF;
            Some(this)
        })
        .collect();

    ChannelSymbols {
        ranges: symbols.try_into().unwrap(),
        startcu,
        endcu,
        count,
    }
}
