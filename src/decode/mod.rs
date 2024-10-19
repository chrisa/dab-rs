mod viterbi;

pub use viterbi::new_viterbi;
pub use viterbi::Viterbi;

const K: i32 = 1536;

pub fn bit_reverse(bits: &mut [u8; 3072]) {
    for chunk in bits.chunks_mut(16) {
        chunk.reverse();
    }
}

pub fn bytes_to_bits(bytes: &[u8; 384]) -> [u8; 3072] {
    let mut bits = [0u8; 3072];

    for i in 0..384 {
        for j in 0..8 {
            bits[i * 8 + j] = (bytes[i] >> j) & 1;
        }
    }

    bits
}

pub fn bits_to_bytes(bits: &[u8; 256]) -> [u8; 30] {
    let mut i = 0;
    let mut j = 0;
    let mut result: [u8; 30] = [0; 30];
    loop {
        result[j] = (bits[i]<<7) + (bits[i+1]<<6) + (bits[i+2]<<5) + (bits[i+3]<<4) +       //be
        (bits[i+4]<<3) + (bits[i+5]<<2) + (bits[i+6]<<1) + bits[i+7];

        j += 1;
        i += 8;
        if i >= 240 {
            break;
        }
    }

    result
}

pub fn qpsk_symbol_demapper(bits: [u8; 3072]) -> [u8; 3072] {
    let mut slice = [0u8; 3072];

    for n in 0..K as usize {
        slice[n] = bits[2 * n];
        slice[n + K as usize] = bits[(2 * n) + 1];
    }

    slice
}

pub fn depuncture(bits: [u8; 2304]) -> [u8; 3096] {
    // 21 blocks, using puncture 1110 1110 1110 1110 1110 1110 1110 1110
    //  3 blocks, using puncture 1110 1110 1110 1110 1110 1110 1110 1100
    // 24 bits,   using puncture 1100 1100 1100 1100 1100 1100
    let mut i: usize = 0;
    let mut k: usize = 0;
    let mut result = [0u8; 3096];

    loop {
        for j in 0..8 {
            result[i + j * 4] = bits[k];
            result[i + j * 4 + 1] = bits[k + 1];
            result[i + j * 4 + 2] = bits[k + 2];
            result[i + j * 4 + 3] = 8; // mark depunctured bit for soft decision
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
            result[i + j * 4 + 3] = 8;
            k += 3;
        }

        let j = 7; // value of j after the loop above (!)
        result[i + j * 4] = bits[k];
        result[i + j * 4 + 1] = bits[k + 1];
        result[i + j * 4 + 2] = 8;
        result[i + j * 4 + 3] = 8;
        k += 2;

        i += 32;
        if i >= 24 * 128 {
            break;
        }
    }

    for j in 0..6 {
        result[i + j * 4] = bits[k];
        result[i + j * 4 + 1] = bits[k + 1];
        result[i + j * 4 + 2] = 8;
        result[i + j * 4 + 3] = 8;
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
pub fn scramble(bits: [u8; 768]) -> [u8; 768] {
    let mut v: u16 = 0x1ff;
    let mut result: [u8; 768] = [0; 768];

    for i in 0..768 {
        v <<= 1;
        let v0 = ((v >> 9) & 1) ^ ((v >> 5) & 1);
        v |= v0;

        result[i] = bits[i] ^ v0 as u8;
    }

    result
}

const CRC_POLY: u32 = 0x8408;
const CRC_GOOD: u32 = 0xf0b8;

pub fn crc16(bits: &[u8; 256]) -> bool {
    let mut crc = 0xffff;

    for bit in bits {
        let c15 = (crc & 1) ^ (bit & 1) as u32;
        crc >>= 1;
        if c15 == 1 {
            crc ^= CRC_POLY;
        }
    }

    crc == CRC_GOOD
}
