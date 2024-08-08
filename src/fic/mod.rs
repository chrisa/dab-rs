use crate::wavefinder::Buffer;

#[derive(Debug)]
pub struct FastInformationChannelBuffer {
    symbol: u8,
    frame: u8,
    bytes: [u8; 384],
}

impl TryFrom<&Buffer> for FastInformationChannelBuffer {
    type Error = ();

    fn try_from(buffer: &Buffer) -> Result<Self, Self::Error> {
        let symbol = buffer.bytes[2];
        if symbol == 2 || symbol == 3 || symbol == 4 {
            let frame = buffer.bytes[3];
            let mut bytes: [u8; 384] = [0; 384];
            bytes.clone_from_slice(&buffer.bytes[12..396]);
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

impl FastInformationChannelBuffer {
    pub fn data(&self) -> &[u8] {
        &self.bytes
    }
    pub fn symbol(&self) -> u8 {
        self.symbol
    }
    pub fn frame(&self) -> u8 {
        self.frame
    }
}
