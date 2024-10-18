use std::fmt;

use crate::{
    decode::{bit_reverse, byte_to_bit, new_viterbi, Fic, Fic2304, Viterbi},
    fic::new_frame,
};

use super::{FastInformationChannelBuffer, FastInformationChannelFrame, FIC_BUFFER};

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
                    _ => format!("wut?"),
                };
                s.push_str(&frame_string);
            }
        }
        write!(f, "{}", s)
    }
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

        if !self.decode_and_crc(&mut frame) {
            println!("oh no frame {:?} failed crc, deleting", frame.frame_number);
            self.frames[frame.frame_number as usize] = None;
        }

        dbg!(self);
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

    fn decode_and_crc(&self, mut frame: &FastInformationChannelFrame) -> bool {
        let syms = frame.bytes.map(byte_to_bit);
        for mut sym in syms {
            bit_reverse(&mut sym);
            self.viterbi.frequency_deinterleave(&mut sym);
            self.viterbi.qpsk_symbol_demapper(&mut sym);
        }

        let mut merged: Fic = [0u8; 1152].into();
        for i in 0..3 {
            merged[(i * 3072)..((i + 1) * 3072)].copy_from_bitslice(&syms[i]);
        }

        let mut split: Fic2304= [0u8; 288].into();
        for i in 0..4 {
            split.copy_from_bitslice(&merged[(i * 2304)..((i + 1) * 2304)]);
            // let depunctured = self.viterbi.depuncture(&split);
            // let viterbied = self.viterbi.viterbi(&depunctured);
        }

        
        
        true
    }
}
