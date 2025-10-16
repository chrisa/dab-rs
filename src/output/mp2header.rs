use bitvec::prelude::*;

/// Represents an MPEG-1/2 Layer II header (C struct equivalent)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mp2Header {
    pub syncword: u16,        // 12 bits
    pub id: bool,             // 1 bit
    pub layer: u8,            // 2 bits
    pub protection_bit: bool, // 1 bit
    pub bit_rate_index: u8,   // 4 bits
    pub sampling_freq: u8,    // 2 bits
    pub padding_bit: bool,    // 1 bit
    pub private_bit: bool,    // 1 bit
    pub mode: u8,             // 2 bits
    pub mode_extension: u8,   // 2 bits
    pub copyright: bool,      // 1 bit
    pub orig: bool,           // 1 bit
    pub emphasis: u8,         // 2 bits
}

impl Mp2Header {
    /// Parses a 32-bit header value into an `Mp2Header` struct.
    pub fn from_u32(bits: u32) -> Self {
        // Create a BitArray (32 bits, MSB-first as in MPEG spec)
        let bits = bits.view_bits::<Msb0>();

        Self {
            syncword: bits[0..12].load_be(),
            id: bits[12],
            layer: bits[13..15].load_be(),
            protection_bit: bits[15],
            bit_rate_index: bits[16..20].load_be(),
            sampling_freq: bits[20..22].load_be(),
            padding_bit: bits[22],
            private_bit: bits[23],
            mode: bits[24..26].load_be(),
            mode_extension: bits[26..28].load_be(),
            copyright: bits[28],
            orig: bits[29],
            emphasis: bits[30..32].load_be(),
        }
    }

    /// Converts the struct back into a 32-bit integer representation.
    pub fn to_u32(&self) -> u32 {
        let mut bits = bitarr!(u32, Msb0; 0, 32);

        bits[0..12].store_be(self.syncword);
        bits.set(12, self.id);
        bits[13..15].store_be(self.layer);
        bits.set(15, self.protection_bit);
        bits[16..20].store_be(self.bit_rate_index);
        bits[20..22].store_be(self.sampling_freq);
        bits.set(22, self.padding_bit);
        bits.set(23, self.private_bit);
        bits[24..26].store_be(self.mode);
        bits[26..28].store_be(self.mode_extension);
        bits.set(28, self.copyright);
        bits.set(29, self.orig);
        bits[30..32].store_be(self.emphasis);

        bits.load_be()
    }
}