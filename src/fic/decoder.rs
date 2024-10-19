use std::fmt;

use crate::{
    decode::{
        bit_reverse, bits_to_bytes, bytes_to_bits, crc16, depuncture, new_viterbi,
        qpsk_symbol_demapper, scramble, Viterbi,
    },
    fic::new_frame,
};

use super::{FastInformationChannelBuffer, FastInformationChannelFrame};

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

fn dump_ascii(bytes: &[char], name: &str) {
    print!("{:?} = ", name);
    for i in 0..32 {
        if bytes[i] > 0x20 as char && bytes[i] < 0x80 as char {
            print!("{}", bytes[i]);
        } else {
            print!(" ");
        }
    }
    println!();
}

impl FastInformationChannelDecoder {
    pub fn try_buffer(&mut self, buffer: FastInformationChannelBuffer) {
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
            return;
        }

        if frame.next_symbol == buffer.symbol {
            self.append_data(&mut frame, &buffer);
            self.frames[buffer.frame as usize] = Some(frame);
        }

        if frame.next_symbol > 4 && !self.decode_and_crc(&mut frame) {
            println!("oh no frame {:?} failed crc, deleting", frame.frame_number);
            self.frames[frame.frame_number as usize] = None;
        }
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

    fn decode_and_crc(&self, frame: &FastInformationChannelFrame) -> bool {
        let mut merged: [u8; 9216] = [0u8; 9216];

        for (i, sym) in frame.bytes.iter().enumerate() {
            let mut bits = bytes_to_bits(sym);
            bit_reverse(&mut bits);
            let bits = self.viterbi.frequency_deinterleave(bits);
            let bits = qpsk_symbol_demapper(bits);
            merged[(i * 3072)..((i + 1) * 3072)].copy_from_slice(&bits);
        }

        let mut split = [0u8; 2304];
        let mut fics: [[u8; 768]; 4] = [[0; 768]; 4];

        for i in 0..4 {
            split.copy_from_slice(&merged[(i * 2304)..((i + 1) * 2304)]);
            let depunctured = depuncture(split);
            let viterbied = self.viterbi.viterbi(depunctured);
            let scrambled = scramble(viterbied);
            fics[i].copy_from_slice(&scrambled);
        }

        let mut fibs: [[u8; 256]; 12] = [[0; 256]; 12];
        for i in 0..256 {
            fibs[0][i] = fics[0][i];
            fibs[1][i] = fics[0][256 + i];
            fibs[2][i] = fics[0][512 + i];

            fibs[3][i] = fics[1][i];
            fibs[4][i] = fics[1][256 + i];
            fibs[5][i] = fics[1][512 + i];

            fibs[6][i] = fics[2][i];
            fibs[7][i] = fics[2][256 + i];
            fibs[8][i] = fics[2][512 + i];

            fibs[9][i] = fics[3][i];
            fibs[10][i] = fics[3][256 + i];
            fibs[11][i] = fics[3][512 + i];
        }

        let mut fib_chars: [[char; 32]; 12] = [[0 as char; 32]; 12];
        for i in 0..12 {
            // Check CRC
            if !crc16(&fibs[i]) {
                continue;
            }

            // If OK, convert to bytes
            let bytes = bits_to_bytes(&fibs[i]);
            for j in 0..32 {
                fib_chars[i][j] = bytes[j] as char;
            }
            dump_ascii(&fib_chars[i], format!("fib #{}", i).as_str());
        }

        true
    }
}
