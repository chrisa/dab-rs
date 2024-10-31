use itertools::Itertools;
//use pretty_hex::*;
use std::fmt;

use crate::{
    decode::{
        bit_reverse, bits_to_bytes, bytes_to_bits, crc16, depuncture, new_viterbi,
        qpsk_symbol_demapper, scramble, Viterbi,
    },
    fic::new_frame,
};

use super::{
    fig::{fig_header, Fig},
    FastInformationBlock, FastInformationChannelBuffer, FastInformationChannelFrame,
};

pub struct FastInformationChannelDecoder {
    frames: Box<[Option<FastInformationChannelFrame>; 32]>,
    viterbi: Viterbi,
}

pub fn new_decoder() -> FastInformationChannelDecoder {
    FastInformationChannelDecoder {
        frames: Box::new([Option::None; 32]),
        viterbi: new_viterbi(),
    }
}

impl fmt::Debug for FastInformationChannelDecoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        for i in 0..32 {
            if let Some(frame) = self.frames[i] {
                let frame_string = match frame.next_symbol {
                    2 => format!("{:?}[] ", frame.frame_number),
                    3 => format!("{:?}[2] ", frame.frame_number),
                    4 => format!("{:?}[23] ", frame.frame_number),
                    5 => format!("{:?}[234] ", frame.frame_number),
                    _ => "wut?".to_string(),
                };
                s.push_str(&frame_string);
            }
        }
        write!(f, "{}", s)
    }
}

impl FastInformationChannelDecoder {
    pub fn try_buffer(
        &mut self,
        buffer: FastInformationChannelBuffer,
    ) -> Option<Vec<FastInformationBlock>> {
        let mut frame;

        if let Some(f) = self.frames[buffer.frame as usize] {
            frame = f;
        } else if buffer.symbol == 2 {
            frame = new_frame(buffer.frame);
        } else {
            println!(
                "can't handle frame {:?} symbol {:?} right now",
                buffer.frame, buffer.symbol
            );
            return None;
        }

        if frame.next_symbol == buffer.symbol {
            self.append_data(&mut frame, &buffer);
            self.frames[buffer.frame as usize] = Some(frame);
        }

        if frame.next_symbol > 4 {
            if let Ok(blocks) = self.decode_and_crc(&frame) {
                return Some(blocks);
            } else {
                // CRC check failed
                println!("frame {:?} failed crc", frame.frame_number);
                self.frames[frame.frame_number as usize] = None;
                return None;
            }
        }

        // Not enough symbols yet
        None
    }

    fn append_data(
        &self,
        frame: &mut FastInformationChannelFrame,
        buffer: &FastInformationChannelBuffer,
    ) {
        // symbols are 2, 3 and 4 -> array indexes 0, 1, 2
        frame.bytes[(buffer.symbol - 2) as usize].copy_from_slice(&buffer.bytes);
        frame.next_symbol = buffer.symbol + 1;
    }

    fn decode_and_crc(
        &self,
        frame: &FastInformationChannelFrame,
    ) -> Result<Vec<FastInformationBlock>, &'static str> {
        let mut merged: [bool; 9216] = [false; 9216];

        for (i, sym) in frame.bytes.iter().enumerate() {
            let mut bits = bytes_to_bits(sym);
            bit_reverse(&mut bits);
            let bits = self.viterbi.frequency_deinterleave(&bits);
            let bits = qpsk_symbol_demapper(&bits);
            merged[(i * 3072)..((i + 1) * 3072)].copy_from_slice(&bits);
        }

        let mut split = [false; 2304];
        let mut fibs: [[bool; 256]; 12] = [[false; 256]; 12];

        for i in 0..4 {
            split.copy_from_slice(&merged[(i * 2304)..((i + 1) * 2304)]);
            let depunctured = depuncture(&split);
            let viterbied = self.viterbi.viterbi(&depunctured);
            let scrambled = scramble(&viterbied);
            // Split into FIBs
            for j in 0..3 {
                fibs[i * 3 + j].copy_from_slice(&scrambled[(j * 256)..(j * 256 + 256)]);
            }
        }

        let mut fib_bytes: [[u8; 30]; 12] = [[0_u8; 30]; 12];

        for i in 0..12 {
            // Check CRC
            if !crc16(&fibs[i]) {
                return Err("crc check failed");
            }

            // If OK, convert to bytes
            let bytes = bits_to_bytes(&fibs[i]);
            fib_bytes[i].copy_from_slice(&bytes);
        }

        let blocks = fib_bytes
            .map(|bytes| FastInformationBlock {
                bytes,
                num: frame.frame_number,
            })
            .to_vec();
        Ok(blocks)
    }

    pub fn extract_figs(&self, fib: &FastInformationBlock) -> Vec<Fig> {
        // println!("fib num: {:?}\n{}", fib.num, pretty_hex(&fib.bytes));

        let fig_iter = fib.bytes.iter().batching(|it| {
            if let Some(h) = it.next() {
                // end marker
                if *h == 0xff {
                    return None;
                }
                if let Some(mut fig) = fig_header(*h) {
                    let body = it.take(fig.header.len);
                    fig.push_data(body.copied().collect());
                    return Some(fig);
                }
            }
            None
        });
        fig_iter.collect()
    }
}
