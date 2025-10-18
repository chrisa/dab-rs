// Rust 1.90 - optimized Viterbi (rate 1/4, k=7, N=4)
// Public API uses &[u8] for symbols (0/1). Use helpers to convert from Vec<bool> if needed.

use libm::erf;
use std::f64::consts::SQRT_2;

/* Constraint length */
const N: usize = 4;
/* Number of symbols per data bit */
const K: usize = 7;

const LONGBITS: usize = 32;
const LOGLONGBITS: usize = 5;
const D: usize = 1 << (K - LOGLONGBITS - 1);

// Derived sizes
const STATES: usize = 1 << (K - 1); // 64
const SYMS_SZ: usize = 1 << K; // 128
const METS_SZ: usize = 1 << N; // 16
const TABLE49_LEN: usize = 1536; // computed from original algorithm

pub struct Viterbi {
    table49: Vec<i32>, // length TABLE49_LEN (1536)
    syms: Vec<usize>,  // length SYMS_SZ (128)
}

pub fn new_viterbi() -> Viterbi {
    let mut v = Viterbi {
        table49: vec![0i32; TABLE49_LEN],
        syms: vec![0usize; SYMS_SZ],
    };
    v.gen_table49();
    v.vd_init();

    v
}

// Normal function integrated from -Inf to x. Range: 0-1
#[allow(dead_code)]
fn normal(x: f64) -> f64 {
    0.5 + 0.5 * erf(x / SQRT_2)
}

// parity using 8-bit lookup
fn parity(i: usize) -> usize {
    let mut x = i as u32;
    x ^= x >> 16;
    x ^= x >> 8;
    PARTAB[(x & 0xff) as usize] as usize
}

/* 8-bit parity lookup table (u8) */
const PARTAB: [u8; 256] = [
    0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1,
    1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0,
    1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0,
    0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1,
    1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0,
    0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1,
    0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1,
    1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0,
];

const POLYS: [usize; 4] = [0x6d, 0x4f, 0x53, 0x6d]; /* k = 7; DAB */

#[derive(Debug, Copy, Clone)]
pub enum Bit {
    False = 0,
    Erased = 1,
    True = 2,
}

impl Bit {
    pub fn from_u8(bit: &u8) -> Bit {
        match bit {
            0 => Bit::False,
            1 => Bit::True,
            u => {
                panic!("unexpected bit value: {}", u)
            }
        }
    }
}

impl Viterbi {
    // initialize symbol mapping
    fn vd_init(&mut self) {
        // syms[i] = combined symbol (4 bits -> index into mets)
        for i in 0..SYMS_SZ {
            let mut sym = 0usize;
            for &p in POLYS.iter() {
                sym = (sym << 1) + parity(i & p);
            }
            self.syms[i] = sym;
        }
    }

    // generate the interleaver table used by original code
    fn gen_table49(&mut self) {
        // KI seq of length 2048
        let mut ki = [0i32; 2048];
        ki[0] = 0;
        for i in 1..2048 {
            ki[i] = (13 * ki[i - 1] + 511) % 2048;
        }

        let mut n = 0usize;
        for &k in ki.iter() {
            if (256..=1792).contains(&k) && (k != 1024) {
                // table49 has length 1536
                if n < TABLE49_LEN {
                    self.table49[n] = k - 1024;
                    n += 1;
                } else {
                    break;
                }
            }
        }
    }

    /// Frequency deinterleave.
    /// Input &bits are bytes 0/1; returns a Vec<u8> (0/1).
    pub fn frequency_deinterleave(&self, bits: &[u8]) -> Vec<u8> {
        // bits expected to be at least 2 * TABLE49_LEN long (original code used 3072-ish).
        let total = bits.len();
        let mut out = vec![0u8; total];

        // constants (as in original)
        let k1 = 1536usize;

        // safety: ensure expected lengths; panic early rather than UB.
        assert!(self.table49.len() >= k1);
        assert!(total >= 2 * k1);

        // Use raw pointers in the hot loop to avoid repeated bounds checks.
        // We still keep safety asserts above.
        unsafe {
            let in_ptr = bits.as_ptr();
            let out_ptr = out.as_mut_ptr();

            for i in 0..k1 {
                let mut k = self.table49[i] as isize;

                // adjust per original logic
                k += if k < 0 {
                    (k1 / 2) as isize
                } else if k > 0 {
                    (k1 / 2 - 1) as isize
                } else {
                    0
                };

                // compute byte offsets (2 * i), (2 * k)
                let dst0 = out_ptr.add(2 * i);
                let src0 = in_ptr.add(2 * k as usize);

                // copy two bytes
                // copy src[0], src[1] -> dst[0], dst[1]
                // using ptr::read avoids bounds checks
                std::ptr::write(dst0, std::ptr::read(src0));
                std::ptr::write(dst0.add(1), std::ptr::read(src0.add(1)));
            }
        }

        out
    }

    /// Viterbi decoder core.
    pub fn viterbi(&self, bits: &[Bit]) -> Vec<u8> {
        let nbits = bits.len() / N - (K - 1);

        // output
        let mut result = vec![0u8; nbits];

        // work arrays
        let mut mets = [0i32; METS_SZ];
        // path storage (each entry stores 32 decisions packed into u32 words)
        let paths_len = (nbits + K - 1) * D;
        let mut paths = vec![0u32; paths_len];

        let mut cmetric = vec![i32::MIN / 4; STATES]; // large negative initial (safe margin)
        let mut nmetric = vec![0i32; STATES];

        // start and end state (original code zeroed and masked)
        let mut startstate: usize = 0;
        let mut endstate: usize = 0;

        startstate &= !((1 << (K - 1)) - 1);
        endstate &= !((1 << (K - 1)) - 1);

        // initialize starting metrics
        for v in cmetric.iter_mut() {
            *v = -999_999;
        }
        cmetric[startstate] = 0;

        // offsets and mask used for packing path decisions
        let mut path_offset: usize = 0;
        let mut symbol_offset: usize = 0;
        let mut mask: u32 = 1;

        // branch metrics mapping: metrics[tx][rx]
        // mapping from bits (tx) and received bit (0/1)
        let metrics: [[i32; 3]; 2] = [[3, 0, -7], [-7, 0, 3]];

        // main decode loop over NBITS symbols
        let mut bitcnt_isize: isize = -((K as isize) - 1); // replicates original init
        loop {
            // compute mets (16 branch metrics) from the next N bits
            // mets index is 0..16, constructed from bits[symbol_offset .. symbol_offset+N-1]
            // compute using bitpacking
            for (i, met) in mets.iter_mut().enumerate() {
                let mut acc = 0i32;
                // build from MSB to LSB to match original
                for j in 0..N {
                    let bindex = symbol_offset + j;
                    let bit_idx = (i >> (N - j - 1)) & 1;
                    acc += metrics[bit_idx][bits[bindex] as usize];
                }
                *met = acc;
            }
            symbol_offset += N;

            // Add-compare-select: compute nmetric from cmetric and mets
            // We'll iterate pairs (i, i+1) and update mask/paths similarly to original.
            let mut i = 0usize;
            while i < STATES {
                // Use local variables to enable optimisation and help auto-vectoriser.
                let c_half = cmetric[i / 2];
                // the other half offset
                let c_half_other = cmetric[(i / 2) + (1 << (K - 2))];

                // two branch metrics
                let b1 = mets[self.syms[i]];
                let b2 = mets[self.syms[i + 1]];

                // compute candidate metrics
                let m0 = c_half + b1;
                let m1 = c_half_other + b2;

                // pick winner and set path bits accordingly
                // first state (i)
                if m1 > m0 {
                    nmetric[i] = m1;
                    // set single bit in paths[path_offset]
                    paths[path_offset] |= mask;
                } else {
                    nmetric[i] = m0;
                }

                // second state (i+1) - note original manip used algebraic rearrangement to re-use work
                // compute alternative candidates
                // m0' = c_half + b2  (note original code used m0 -= b1 then assigned)
                let alt0 = c_half + b2;
                let alt1 = c_half_other + b1;

                if alt1 > alt0 {
                    nmetric[i + 1] = alt1;
                    paths[path_offset] |= mask << 1;
                } else {
                    nmetric[i + 1] = alt0;
                }

                // advance mask and maybe path_offset
                mask <<= 2;
                if mask == 0 {
                    mask = 1;
                    path_offset += 1;
                }

                i += 2;
            }

            if mask != 1 {
                // we used some bits of the current word: move to next word for the next round
                path_offset += 1;
            }

            bitcnt_isize += 1;
            if bitcnt_isize == nbits as isize {
                break;
            }

            // roll metrics
            cmetric.copy_from_slice(&nmetric);
        }

        // Chain back from terminal state to produce decoded data (reverse trace)
        // original used (endstate >> LOGLONGBITS) indexing into path words and bit-check on
        // (1u32 << (endstate & (LONGBITS - 1)))
        // replicate that logic safely
        for i in (0..nbits).rev() {
            // step back path_offset by D
            path_offset -= D;

            let word_index = path_offset + (endstate >> LOGLONGBITS);
            let bit_pos = (endstate & (LONGBITS - 1)) as u32;
            let word = paths[word_index];

            if (word & (1u32 << bit_pos)) != 0 {
                endstate |= 1 << (K - 1);
                result[i] = 1u8;
            } else {
                result[i] = 0u8;
            }
            endstate >>= 1;
        }

        result
    }
}
