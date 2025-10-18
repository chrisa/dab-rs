mod viterbi;

use itertools::Itertools;
pub use viterbi::new_viterbi;
pub use viterbi::{Bit, Viterbi};

const K: i32 = 1536;

pub fn bit_reverse(bits: &mut [u8]) {
    assert!(bits.len().is_multiple_of(16));
    for chunk in bits.chunks_mut(16) {
        chunk.reverse();
    }
}

pub fn bytes_to_bits(bytes: &[u8]) -> Vec<u8> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);

    for byte in bytes {
        for j in 0..8 {
            bits.push(if (byte >> j) & 1 > 0 { 1 } else { 0 });
        }
    }

    bits
}

pub fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    assert!(bits.len().is_multiple_of(8));
    let mut bytes = Vec::with_capacity(bits.len() / 8);

    for mut chunk in &bits.iter().chunks(8) {
        bytes.push(
            (chunk.next().unwrap()<<7) + (chunk.next().unwrap()<<6) + (chunk.next().unwrap()<<5) + (chunk.next().unwrap()<<4) + //be
            (chunk.next().unwrap()<<3) + (chunk.next().unwrap()<<2) + (chunk.next().unwrap()<<1) +  chunk.next().unwrap()
        );
    }

    bytes
}

pub fn qpsk_symbol_demapper(bits: &[u8]) -> Vec<u8> {
    let mut slice = vec![0; bits.len()];

    for n in 0..K as usize {
        slice[n] = bits[2 * n];
        slice[n + K as usize] = bits[(2 * n) + 1];
    }

    slice
}

pub fn depuncture(bits: &[u8; 2304]) -> Vec<Bit> {
    // 21 blocks, using puncture 1110 1110 1110 1110 1110 1110 1110 1110
    //  3 blocks, using puncture 1110 1110 1110 1110 1110 1110 1110 1100
    // 24 bits,   using puncture 1100 1100 1100 1100 1100 1100
    let mut i: usize = 0;
    let mut k: usize = 0;
    let mut result: Vec<Bit> = vec![Bit::Erased; 3096];

    loop {
        for j in 0..8 {
            result[i + j * 4] = Bit::from_u8(&bits[k]);
            result[i + j * 4 + 1] = Bit::from_u8(&bits[k + 1]);
            result[i + j * 4 + 2] = Bit::from_u8(&bits[k + 2]);
            result[i + j * 4 + 3] = Bit::Erased; // mark depunctured bit for soft decision
            k += 3;
        }

        i += 32;
        if i >= 21 * 128 {
            break;
        }
    }

    let mut i = 21 * 128;
    loop {
        for j in 0..7 {
            result[i + j * 4] = Bit::from_u8(&bits[k]);
            result[i + j * 4 + 1] = Bit::from_u8(&bits[k + 1]);
            result[i + j * 4 + 2] = Bit::from_u8(&bits[k + 2]);
            result[i + j * 4 + 3] = Bit::Erased;
            k += 3;
        }

        let j = 7; // value of j after the loop above (!)
        result[i + j * 4] = Bit::from_u8(&bits[k]);
        result[i + j * 4 + 1] = Bit::from_u8(&bits[k + 1]);
        result[i + j * 4 + 2] = Bit::Erased;
        result[i + j * 4 + 3] = Bit::Erased;
        k += 2;

        i += 32;
        if i >= 24 * 128 {
            break;
        }
    }

    for j in 0..6 {
        result[i + j * 4] = Bit::from_u8(&bits[k]);
        result[i + j * 4 + 1] = Bit::from_u8(&bits[k + 1]);
        result[i + j * 4 + 2] = Bit::Erased;
        result[i + j * 4 + 3] = Bit::Erased;
        k += 2;
    }

    result
}

// pub fn puncture(&self, bits: [u8; 2304]) -> [u8; 3096] {
//     let mut i: usize = 0;
//     let mut k: usize = 0;
//     let mut result = [0u8; 3096];

//     loop {
//         for j in 0..8 {
//             result[k + 0] = bits[i + j*4 + 0];
//             result[k + 1] = bits[i + j*4 + 1];
//             result[k + 2] = bits[i + j*4 + 2];
//             k += 3;
//         }

//         i += 32;
//         if i > 24 * 128 {
//             break;
//         }
//     }

//     let mut i = 21 * 128;
//     loop {
//         for j in 0..7 {
//             result[k + 0] = bits[i + j*4 + 0];
//             result[k + 1] = bits[i + j*4 + 1];
//             result[k + 2] = bits[i + j*4 + 2];
//             k += 3;
//         }

//         let j = 7; // value of j after the loop above (!)
//         result[k + 0] = bits[i + j*4 + 0];
//         result[k + 1] = bits[i + j*4 + 1];
//         k += 2;

//         i += 32;
//         if i > 24 * 128 {
//             break;
//         }
//     }

//     for j in 0..6 {
//         result[k + 0] = bits[i + j*4 + 0];
//         result[k + 1] = bits[i + j*4 + 1];
//         k += 2;
//     }

//     result
// }

//10 Energy dispersal
//10.1 General procedure
//10.2 Energy dispersal as applied in the Fast Information Channel
pub fn scramble(bits: &[u8]) -> Vec<u8> {
    let mut v: u16 = 0x1ff;
    let mut result = vec![];

    for bit in bits {
        v <<= 1;
        let v0 = ((v >> 9) & 1) ^ ((v >> 5) & 1);
        v |= v0;

        let res = ((bit & 1) != 0) ^ ((v0 & 1) != 0);
        result.push(if res { 1 } else { 0 });
    }

    result
}

const CRC_POLY: u32 = 0x8408;
const CRC_GOOD: u32 = 0xf0b8;

pub fn crc16(bits: &[u8; 256]) -> bool {
    let mut crc = 0xffff;

    for bit in bits {
        let c15 = (crc & 1) ^ (if (bit & 1) != 0 { 1u32 } else { 0u32 });
        crc >>= 1;
        if c15 == 1 {
            crc ^= CRC_POLY;
        }
    }

    crc == CRC_GOOD
}
