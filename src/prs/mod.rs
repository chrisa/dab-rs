use crate::wavefinder::Buffer;
use rustfft::num_complex::{c64, Complex64};
use std::convert::TryFrom;

mod fft;
mod maths;
mod reference;
mod sync;
pub use sync::new_synchroniser;

pub const PRS_POINTS: usize = 2048;
pub type PhaseReferenceArray = [Complex64; PRS_POINTS];

#[derive(Debug)]
pub struct PhaseReferenceBuffer {
    block: u8,
    bytes: [u8; 512],
}

impl TryFrom<&Buffer> for PhaseReferenceBuffer {
    type Error = ();

    fn try_from(buffer: &Buffer) -> Result<Self, Self::Error> {
        if buffer.bytes[9] == 0x02 {
            let bytes = buffer.bytes;
            let mut prs: [u8; 512] = [0; 512];
            prs.clone_from_slice(&bytes[12..524]);
            Ok(PhaseReferenceBuffer {
                bytes: prs,
                block: bytes[7],
            })
        } else {
            Err(())
        }
    }
}

pub struct PhaseReferenceSymbol {
    next_block: u8,
    bytes: [u8; PRS_POINTS],
}

pub fn new_symbol() -> PhaseReferenceSymbol {
    PhaseReferenceSymbol {
        next_block: 0,
        bytes: [0; PRS_POINTS],
    }
}

impl PhaseReferenceSymbol {
    pub fn try_buffer(&mut self, buffer: &Buffer) {
        if self.is_complete() {
            return;
        }
        if let Ok(prs_buffer) = TryInto::<PhaseReferenceBuffer>::try_into(buffer) {
            if prs_buffer.block == self.next_block {
                println!("PRS block: {:?}", prs_buffer.block);
                self.append_data(&prs_buffer);
            }
        }
    }

    fn append_data(&mut self, buffer: &PhaseReferenceBuffer) {
        let block = buffer.block as usize;
        self.bytes[(block * 512)..((block * 512) + 512)].copy_from_slice(&buffer.bytes);
        self.next_block += 1;
    }

    pub fn is_complete(&self) -> bool {
        self.next_block == 4
    }

    pub fn vector(&self) -> PhaseReferenceArray {
        self.bytes.map(|b| c64(0.0, b as f64 - 128.0))
    }
}
