use core::fmt;

use crate::wavefinder::Buffer;

pub mod decoder;
pub mod ensemble;
pub mod fig;

pub use decoder::new_decoder;

const FIC_BUFFER: usize = 384;

#[derive(Debug, Copy, Clone)]
pub struct FastInformationChannelBuffer {
    symbol: u8,
    frame: u8,
    bytes: [u8; FIC_BUFFER],
}

impl TryFrom<&Buffer> for FastInformationChannelBuffer {
    type Error = ();

    fn try_from(buffer: &Buffer) -> Result<Self, Self::Error> {
        let symbol = buffer.bytes[2];
        if symbol == 2 || symbol == 3 || symbol == 4 {
            let frame = buffer.bytes[3];
            let mut bytes: [u8; FIC_BUFFER] = [0; FIC_BUFFER];
            bytes.clone_from_slice(&buffer.bytes[12..(12 + FIC_BUFFER)]);
            Ok(FastInformationChannelBuffer {
                symbol,
                frame,
                bytes,
            })
        } else {
            Err(())
        }
    }
}

#[derive(Clone, Copy)]
pub struct FastInformationChannelFrame {
    frame_number: u8,
    next_symbol: u8,
    bytes: [[u8; FIC_BUFFER]; 3],
}

impl fmt::Debug for FastInformationChannelFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        for (i, buffer) in self.bytes.iter().enumerate() {
            s.push_str(
                format!(
                    "\nframe number: {:?} next symbol: {:?}\n  sym{:?} = ",
                    self.frame_number, self.next_symbol, i
                )
                .as_str(),
            );
            for byte in buffer.iter().take(FIC_BUFFER) {
                s.push_str(format!("{:02x} ", byte).as_str());
            }
        }
        s.push('\n');
        write!(f, "{}", s)
    }
}

pub fn new_frame(frame_number: u8) -> FastInformationChannelFrame {
    FastInformationChannelFrame {
        frame_number,
        next_symbol: 2,
        bytes: [[0; FIC_BUFFER]; 3],
    }
}

#[derive(Copy, Clone)]
pub struct FastInformationBlock {
    num: u8,
    bytes: [u8; 30],
}

impl fmt::Debug for FastInformationBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        s.push_str(format!("{:?} = ", self.num).as_str());
        for byte in self.bytes {
            if byte > 0x20 && byte < 0x80 {
                s.push(byte as char);
            } else {
                s.push(' ');
            }
        }
        write!(f, "{}", s)
    }
}
