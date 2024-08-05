use crate::wavefinder::Buffer;
use std::convert::TryFrom;

#[derive(Debug)]
pub struct PhaseReferenceBuffer {
    block: u8,
    bytes: [u8; 512],
}

impl TryFrom<Buffer> for PhaseReferenceBuffer {
    type Error = ();

    fn try_from(buffer: Buffer) -> Result<Self, Self::Error> {
        //println!("try_from: {:?}", &buffer.bytes[0..12]);
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
    block_seen: u8,
    bytes: [u8; 2048],
}

pub fn new() -> PhaseReferenceSymbol {
    PhaseReferenceSymbol {
        block_seen: 255,
        bytes: [0; 2048],
    }
}

impl PhaseReferenceSymbol {
    pub fn try_buffer(&mut self, buffer: Buffer) {
        if let Ok(prs_buffer) = TryInto::<PhaseReferenceBuffer>::try_into(buffer) {
            println!("prs_buffer: {:?}", prs_buffer.block);
            if self.block_seen == 255 || self.block_seen < 4 {
                self.append_data(prs_buffer);
            }
        }
    }

    fn append_data(&mut self, buffer: PhaseReferenceBuffer) {
        self.block_seen = buffer.block();
        let block = buffer.block() as usize;
        self.bytes[block..(block + 512)].copy_from_slice(buffer.data());
    }
}
