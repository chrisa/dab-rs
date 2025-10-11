use crate::{fic::ensemble::Service, wavefinder::Buffer};

#[derive(Debug)]
pub struct MainServiceChannel<'a> {
    service: &'a Service,
    symbols: ChannelSymbols,
    buffers: Box<Buffers>,
}

#[derive(Debug)]
pub struct Buffers {
    cur_frame: u8,
    cifcnt: u64,
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
}

#[derive(Debug)]
pub struct ChannelSymbols {
    symbols: [SymbolRange; 4],
    startcu: u16,
    endcu: u16,
    count: u16,
}

#[derive(Debug)]
pub struct MainServiceChannelBuffer {}

pub fn new_channel(service: &Service) -> MainServiceChannel<'_> {
    let symbols = channel_symbols(service);
    MainServiceChannel {
        service,
        symbols,
        buffers: Box::new(Buffers {
            cur_frame: 0,
            cifcnt: 0,
        }),
    }
}

impl<'a> MainServiceChannel<'a> {
    pub fn try_buffer(&mut self, buffer: &Buffer) {
        let symbol = buffer.bytes[2];
        let frame: u8 = buffer.bytes[3];

        if symbol == self.symbols.symbols[0].start {
            self.buffers.as_mut().cur_frame = frame;
            

        }
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
        symbols: symbols.try_into().unwrap(),
        startcu,
        endcu,
        count,
    }
}
