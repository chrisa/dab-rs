mod viterbi;

pub use viterbi::new_viterbi;
pub use viterbi::Viterbi;

const K: i32 = 1536;

pub fn bit_reverse(bits: &mut [bool]) {
    assert!(bits.len() % 16 == 0);
    for chunk in bits.chunks_mut(16) {
        chunk.reverse();
    }
}

pub fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
    let mut bits = vec![];

    for byte in bytes {
        for j in 0..8 {
            bits.push(((byte >> j) & 1) != 0);
        }
    }

    bits
}

fn byte(b: bool) -> u8 {
    if b {
        1
    } else {
        0
    }
}

pub fn bits_to_bytes(bits: &[bool; 256]) -> [u8; 30] {
    let mut i = 0;
    let mut j = 0;
    let mut result: [u8; 30] = [0; 30];
    loop {
        result[j] = (byte(bits[i])<<7) + (byte(bits[i+1])<<6) + (byte(bits[i+2])<<5) + (byte(bits[i+3])<<4) +       //be
        (byte(bits[i+4])<<3) + (byte(bits[i+5])<<2) + (byte(bits[i+6])<<1) + byte(bits[i+7]);

        j += 1;
        i += 8;
        if i >= 240 {
            break;
        }
    }

    result
}

pub fn qpsk_symbol_demapper(bits: &[bool]) -> Vec<bool> {
    let mut slice = vec![];
    slice.resize(bits.len(), false);

    for n in 0..K as usize {
        slice[n] = bits[2 * n];
        slice[n + K as usize] = bits[(2 * n) + 1];
    }

    slice
}

pub fn depuncture(bits: &[bool; 2304]) -> Vec<bool> {
    // 21 blocks, using puncture 1110 1110 1110 1110 1110 1110 1110 1110
    //  3 blocks, using puncture 1110 1110 1110 1110 1110 1110 1110 1100
    // 24 bits,   using puncture 1100 1100 1100 1100 1100 1100
    let mut i: usize = 0;
    let mut k: usize = 0;
    let mut result = vec![];
    result.resize(3096, false);

    loop {
        for j in 0..8 {
            result[i + j * 4] = bits[k];
            result[i + j * 4 + 1] = bits[k + 1];
            result[i + j * 4 + 2] = bits[k + 2];
            result[i + j * 4 + 3] = false; // mark depunctured bit for soft decision
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
            result[i + j * 4] = bits[k];
            result[i + j * 4 + 1] = bits[k + 1];
            result[i + j * 4 + 2] = bits[k + 2];
            result[i + j * 4 + 3] = false;
            k += 3;
        }

        let j = 7; // value of j after the loop above (!)
        result[i + j * 4] = bits[k];
        result[i + j * 4 + 1] = bits[k + 1];
        result[i + j * 4 + 2] = false;
        result[i + j * 4 + 3] = false;
        k += 2;

        i += 32;
        if i >= 24 * 128 {
            break;
        }
    }

    for j in 0..6 {
        result[i + j * 4] = bits[k];
        result[i + j * 4 + 1] = bits[k + 1];
        result[i + j * 4 + 2] = false;
        result[i + j * 4 + 3] = false;
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
pub fn scramble(bits: &[bool]) -> Vec<bool> {
    let mut v: u16 = 0x1ff;
    let mut result = vec![];

    for bit in bits {
        v <<= 1;
        let v0 = ((v >> 9) & 1) ^ ((v >> 5) & 1);
        v |= v0;

        result.push(bit ^ ((v0 & 1) != 0));
    }

    result
}

const CRC_POLY: u32 = 0x8408;
const CRC_GOOD: u32 = 0xf0b8;

pub fn crc16(bits: &[bool; 256]) -> bool {
    let mut crc = 0xffff;

    for bit in bits {
        let c15 = (crc & 1) ^ (if *bit { 1u32 } else { 0u32 });
        crc >>= 1;
        if c15 == 1 {
            crc ^= CRC_POLY;
        }
    }

    crc == CRC_GOOD
}
