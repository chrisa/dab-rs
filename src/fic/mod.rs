use crate::wavefinder::Buffer;

mod decoder;

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

pub fn new_frame(frame_number: u8) -> FastInformationChannelFrame {
    FastInformationChannelFrame {
        frame_number,
        next_symbol: 2,
        bytes: [[0; FIC_BUFFER]; 3],
    }
}
