use crate::fic::ensemble::Service;
use crate::msc::{ChannelSymbols, SymbolRange};

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
