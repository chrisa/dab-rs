use crate::fic::ensemble::Service;

const CUSPERSYM: u16 = 48;
const MSCSTART: u16 = 5;
const SYMSPERCIF: u16 = 18;

#[derive(Debug)]
pub struct SymbolRange {
    start: u16,
    end: u16,
}

impl SymbolRange {
    pub fn length(&self) -> u16 {
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

pub fn channel_symbols(service: &Service) -> ChannelSymbols {
    let subchannel = service.subchannel();

    let size = subchannel.size();
    let start = subchannel.startaddr();
    let startcu = start % CUSPERSYM;
    let endcu = (start + size) % CUSPERSYM;

    let symbol_0 = SymbolRange {
        start: start / CUSPERSYM + MSCSTART,
        end: (start + size) / CUSPERSYM + MSCSTART,
    };
    let count = symbol_0.length() + 1;

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
