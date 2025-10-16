use libm::floor;

use crate::{
    decode::{bit_reverse, bits_to_bytes, bytes_to_bits, qpsk_symbol_demapper, scramble, Viterbi, Bit},
    fic::ensemble::{Protection, SubChannel, SubChannelType},
    msc::{tables::PVEC, Buffers, ChannelSymbols, MainServiceChannelBuffer, SizedBuffer},
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

    pub fn decode(&self, buffers: &SizedBuffer, sc: &dyn SubChannel, sym: &ChannelSymbols) -> Vec<u8> {

        // time disinterleave
        let dis = match buffers {
            SizedBuffer::One(buffers) => self.time_disinterleave::<1>(buffers, sc, sym),
            SizedBuffer::Two(buffers) => self.time_disinterleave::<2>(buffers, sc, sym),
            SizedBuffer::Three(buffers) => self.time_disinterleave::<3>(buffers, sc, sym),
        };

        // use pretty_hex::*;
        // println!("{}", pretty_hex(&dis));

        // depuncture
        let depunctured = match (sc.subchannel_type(), sc.protection()) {
            (SubChannelType::Audio, Protection::EEP) => self.eep_depuncture(&dis, sc),
            (SubChannelType::Audio, Protection::UEP) => self.uep_depuncture(&dis, sc),
            (SubChannelType::Data, _) => self.eep_depuncture_data(&dis, sc),
            (t, p) => panic!("unexpected subchannel configuration: {:?} {:?}", t, p),
        };

        // println!("bits: {}", depunctured.len());

        use pretty_hex::*;
        // println!("{}", pretty_hex(&depunctured));

        let vited = self.viterbi.viterbi(&depunctured);
        // println!("{}", pretty_hex(&vited));
        let scrambled = scramble(&vited);
        // println!("{}", pretty_hex(&scrambled));
        let out = bits_to_bytes(&scrambled);
        // println!("{}", pretty_hex(&out));

        // -> PAD, MPEG
        out
    }

    // /*
    // ** Time disinterleaving ETSI EN 300 401 V1.3.3 (2001-05), 12, P.161
    // ** Multiplex reconfiguration has not yet been considered.
    // **
    // ** Writes a single logical frame to obuf.
    // */
    // int time_disinterleave(struct cbuf *cbuf, unsigned char *obuf, int subchsz, struct symrange *sr)
    // {
    // 	int i, cif;
    // 	const int map[] = {0,8,4,12,2,10,6,14,1,9,5,13,3,11,7,15};

    // 	for (i=0; i < (subchsz * BITSPERCU); i++) {
    // 		cif = map[i % 16];
    // 		*(obuf+i) = *(cbuf->cb[(cbuf->head + cif) % 16].data + BITSPERCU * sr->startcu + i);
    // 	}
    // 	return 0;
    // }

    fn time_disinterleave<const N: usize>(
        &self,
        buffers: &Buffers<N>,
        sc: &dyn SubChannel,
        sym: &ChannelSymbols,
    ) -> Vec<u8> {
        const TD_MAP: [usize; 16] = [0, 8, 4, 12, 2, 10, 6, 14, 1, 9, 5, 13, 3, 11, 7, 15];
        const BITSPERCU: u16 = 64;
        let mut result = Vec::<u8>::with_capacity(sc.size() as usize * BITSPERCU as usize);

        use pretty_hex::*;

        for i in 0..(sc.size() * BITSPERCU) as usize {
            let cif = TD_MAP[i % 16];
            let offset = (BITSPERCU * sym.startcu) as usize + i;
            let n = floor(offset as f64 / 3072.0f64) as usize;
            let m = offset % 3072;

            // println!("i: {} offset: {} n: {} m: {}", i, offset, n, m);
            // dbg!(&buffers.symbols);

            if let Some(buf) = buffers.symbols[cif][n] {
                // println!("{}", pretty_hex(&buf.bits));
                let bit = buf.bits[m];
                result.push(bit);
            } else {
                panic!("missing buffer!");
            }
        }

        // println!("{}", pretty_hex(&result));

        result
    }

    fn eep_depuncture(&self, bits: &Vec<u8>, sc: &dyn SubChannel) -> Vec<Bit> {
        // println!("eep got bits of len {}", bits.len());
        Vec::new()
    }

    // int uep_depuncture(unsigned char *obuf, unsigned char *inbuf, struct audio_subch *s, int* len)
    // {
    //     int i, j, k, indx;
    //     const struct uepprof p = ueptable[s->uep_indx];

    //     j = 0;
    //     k = 0;
    //     for (indx=0; indx < 4; indx++)
    //         for (i=0; i < BLKSIZE * p.l[indx]; i++) {
    //             if (pvec[p.pi[indx]][i % 32])
    //                 *(obuf + k++) = OFFSET - 1 + (*(inbuf + j++) << 1);
    //             else
    //                 *(obuf + k++) = OFFSET;
    //         }
    //     /* Depuncture remaining 24 bits using rate 8/16 */
    //     for (i=0; i < 24; i++)
    //         if (pvec[7][i % 32])
    //             *(obuf + k++) = OFFSET - 1 + (*(inbuf + j++) << 1);
    //         else
    //             *(obuf + k++) = OFFSET;
//     *len = k;
    //     return 0;
    // }

    fn uep_depuncture(&self, bits: &Vec<u8>, sc: &dyn SubChannel) -> Vec<Bit> {
        const BLKSIZE: usize = 128;

        // println!("uep got bits of len {}", bits.len());

        let uep = match sc.uep_profile() {
            Some(p) => p,
            None => panic!("no UEP profile while uep_depuncturing?"),
        };

        // dbg!(uep);

        let mut result: Vec<Bit> = Vec::with_capacity(4 * bits.len());

        let mut iter = bits.iter();

        for indx in 0..4 {
            // println!("indx: {}, uep.l[indx]: {}", indx, uep.l[indx]);
            for i in 0..(BLKSIZE * uep.l[indx]) {
                // println!("i: {}", i);
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

        // println!("uep returning bits of len {}", result.len());

        result
    }

    fn eep_depuncture_data(&self, bits: &Vec<u8>, sc: &dyn SubChannel) -> Vec<Bit> {
        // println!("eep data got bits of len {}", bits.len());
        Vec::new()
    }
}
