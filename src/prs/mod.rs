use crate::wavefinder::Buffer;
use rustfft::num_complex::{c64, Complex64};
use std::collections::HashSet;
use std::convert::TryFrom;

mod reference;
mod sync;
mod maths;
mod fft;
pub use sync::new_synchroniser;

#[derive(Debug)]
pub struct PhaseReferenceBuffer {
    block: u8,
    bytes: [u8; 512],
}

impl TryFrom<Buffer> for PhaseReferenceBuffer {
    type Error = ();

    fn try_from(buffer: Buffer) -> Result<Self, Self::Error> {
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

impl PhaseReferenceBuffer {
    fn data(&self) -> &[u8] {
        &self.bytes
    }
    fn block(&self) -> u8 {
        self.block
    }
}

pub struct PhaseReferenceSymbol {
    blocks_seen: HashSet<u8>,
    bytes: [u8; 2048],
}

pub fn new_symbol() -> PhaseReferenceSymbol {
    PhaseReferenceSymbol {
        blocks_seen: Default::default(),
        bytes: [0; 2048],
    }
}

impl PhaseReferenceSymbol {
    pub fn try_buffer(&mut self, buffer: Buffer) {
        if self.complete() {
            return;
        }
        if let Ok(prs_buffer) = TryInto::<PhaseReferenceBuffer>::try_into(buffer) {
            // println!("prs_buffer: {:?}", prs_buffer.block);
            if self.blocks_seen.is_empty() && prs_buffer.block == 0 {
                self.append_data(&prs_buffer);
            }
            if !self.blocks_seen.is_empty() && prs_buffer.block > 0 {
                self.append_data(&prs_buffer);
            }
        }
    }

    fn append_data(&mut self, buffer: &PhaseReferenceBuffer) {
        self.blocks_seen.insert(buffer.block());
        let block = buffer.block() as usize;
        self.bytes[(block * 512)..((block * 512) + 512)].copy_from_slice(buffer.data());
    }

    pub fn complete(&self) -> bool {
        let blocks: [u8; 4] = [0, 1, 2, 3];
        let blockset = HashSet::from(blocks);
        let diff = blockset.difference(&self.blocks_seen);
        diff.count() == 0
    }

    pub fn vector(&self) -> [Complex64; 2048] {
        self.bytes.map(|b| c64(0.0, b as f64 - 128.0))
    }
}
