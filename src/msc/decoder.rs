use libm::floor;

use crate::{
    decode::{
        Bit, Viterbi, bit_reverse, bits_to_bytes, bytes_to_bits, qpsk_symbol_demapper, scramble,
    },
    fic::ensemble::{Protection, SubChannel, SubChannelType},
    msc::{Buffers, ChannelSymbols, MainServiceChannelBuffer, SizedBuffer, tables::{EEP2A8, PVEC}},
    new_viterbi,
    wavefinder::Buffer,
};
use std::fmt;

pub struct MainServiceChannelDecoder {
    viterbi: Viterbi,
}

impl fmt::Debug for MainServiceChannelDecoder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "decoder")
    }
}

pub fn new_decoder() -> MainServiceChannelDecoder {
    MainServiceChannelDecoder {
        viterbi: new_viterbi(),
    }
}

impl MainServiceChannelDecoder {
    pub fn deinterleave(&self, buffer: &Buffer) -> MainServiceChannelBuffer {
        let mut bits = bytes_to_bits(&buffer.bytes[12..396]);
        bit_reverse(&mut bits);
        let bits = self.viterbi.frequency_deinterleave(&bits);
        let bits = qpsk_symbol_demapper(&bits);
        MainServiceChannelBuffer {
            bits: bits.try_into().expect("didn't get 3072 bits?"),
        }
    }

    pub fn decode(
        &self,
        buffers: &SizedBuffer,
        sc: &dyn SubChannel,
        sym: &ChannelSymbols,
    ) -> Vec<u8> {
        // time disinterleave
        let dis = match buffers {
            SizedBuffer::One(buffers) => self.time_disinterleave::<1>(buffers, sc, sym),
            SizedBuffer::Two(buffers) => self.time_disinterleave::<2>(buffers, sc, sym),
            SizedBuffer::Three(buffers) => self.time_disinterleave::<3>(buffers, sc, sym),
        };

        // depuncture
        let depunctured = match (sc.subchannel_type(), sc.protection()) {
            (SubChannelType::Audio, Protection::EEP) => self.eep_depuncture(&dis, sc),
            (SubChannelType::Audio, Protection::UEP) => self.uep_depuncture(&dis, sc),
            (SubChannelType::Data, _) => self.eep_depuncture_data(&dis, sc),
            (t, p) => panic!("unexpected subchannel configuration: {:?} {:?}", t, p),
        };

        let vited = self.viterbi.viterbi(&depunctured);
        let scrambled = scramble(&vited);
        let bytes = bits_to_bytes(&scrambled);

        // eprintln!("bytes len: {}", bytes.len());

        bytes
    }

    fn time_disinterleave<const N: usize>(
        &self,
        buffers: &Buffers<N>,
        sc: &dyn SubChannel,
        sym: &ChannelSymbols,
    ) -> Vec<u8> {
        const TD_MAP: [usize; 16] = [0, 8, 4, 12, 2, 10, 6, 14, 1, 9, 5, 13, 3, 11, 7, 15];
        const BITSPERCU: u16 = 64;
        let mut result = Vec::<u8>::with_capacity(sc.size() as usize * BITSPERCU as usize);

        for i in 0..(sc.size() * BITSPERCU) as usize {
            let cif = TD_MAP[i % 16];
            let offset = (BITSPERCU * sym.startcu) as usize + i;
            let n = floor(offset as f64 / 3072.0f64) as usize;
            let m = offset % 3072;

            if let Some(buf) = buffers.symbols[(buffers.lframe + cif) % 16][n] {
                let bit = buf.bits[m];
                result.push(bit);
            } else {
                panic!("missing buffer!");
            }
        }

        result
    }

    fn eep_depuncture_data(&self, _bits: &[u8], _sc: &dyn SubChannel) -> Vec<Bit> {
        // println!("eep got bits of len {}", bits.len());
        Vec::new()
    }

    fn uep_depuncture(&self, bits: &[u8], sc: &dyn SubChannel) -> Vec<Bit> {
        const BLKSIZE: usize = 128;

        let uep = match sc.uep_profile() {
            Some(p) => p,
            None => panic!("no UEP profile while uep_depuncturing?"),
        };

        let mut result: Vec<Bit> = Vec::with_capacity(4 * bits.len());
        let mut iter = bits.iter();

        for indx in 0..4 {
            for i in 0..(BLKSIZE * uep.l[indx]) {
                if PVEC[uep.pi[indx]][i % 32] == 1 {
                    result.push(Bit::from_u8(iter.next().unwrap()));
                } else {
                    result.push(Bit::Erased);
                }
            }
        }

        for i in 0..24 {
            if PVEC[7][i % 32] == 1 {
                result.push(Bit::from_u8(iter.next().unwrap()));
            } else {
                result.push(Bit::Erased);
            }
        }

        result
    }

    fn eep_depuncture(&self, bits: &[u8], sc: &dyn SubChannel) -> Vec<Bit> {
        const BLKSIZE: usize = 128;

        let eep = if sc.bitrate() == 8 && sc.protlvl() == 1 {
            EEP2A8
        } else {
            match sc.eep_profile() {
                Some(p) => p,
                None => panic!("no EEP profile while eep_depuncturing?"),
            }
        };

        let n: i16 = sc.size() as i16 / eep.sizemul as i16;
        assert!(n >= 3);

        let mut result: Vec<Bit> = Vec::with_capacity(4 * bits.len());
        let mut iter = bits.iter();

        for indx in 0..2 {
            for i in 0..(BLKSIZE * ((eep.l[indx].mul as usize * n as usize) + eep.l[indx].offset as usize)) {
                if PVEC[eep.pi[indx]][i % 32] == 1 {
                    result.push(Bit::from_u8(iter.next().unwrap()));
                } else {
                    result.push(Bit::Erased);
                }
            }
        }

        for i in 0..24 {
            if PVEC[7][i % 32] == 1 {
                result.push(Bit::from_u8(iter.next().unwrap()));
            } else {
                result.push(Bit::Erased);
            }
        }

        result
    }
}
