use libm::erf;
use std::f64::consts::SQRT_2;

use super::{Fic2304, Fic3096, Fic768, FicSymbol};
pub struct Viterbi {
    metrics: Box<[[i32; 256]; 2]>,
    table49: Box<[i32; 2048]>,
}

pub fn new_viterbi() -> Viterbi {
    let mut v = Viterbi {
        metrics: Box::new([[0; 256]; 2]),
        table49: Box::new([0; 2048]),
    };
    v.gen_metrics();
    v.gen_table49();
    v
}

const OFFSET: f64 = 128.0;
const K: i32 = 1536;

// Normal function integrated from -Inf to x. Range: 0-1
fn normal(x: f64) -> f64 {
    0.5 + 0.5 * erf(x / SQRT_2)
}

impl Viterbi {
    fn gen_metrics(&mut self) {
        let amp: f64 = 1.0;
        let noise: f64 = 1.0;
        let bias: f64 = 0.0;
        let scale: f64 = 4.0;

        let mut metrics: [[f64; 256]; 2] = [[0.0; 256]; 2];

        {
            let p1 = normal(((0.0 - OFFSET + 0.5) / amp - 1.0) / noise);
            let p0 = normal(((0.0 - OFFSET + 0.5) / amp + 1.0) / noise);
            metrics[0][0] = (2.0 * p0 / (p1 + p0)).log2() - bias;
            metrics[1][0] = (2.0 * p1 / (p1 + p0)).log2() - bias;
        }

        for s in 1..255 {
            let p1 = normal(((s as f64 - OFFSET + 0.5) / amp - 1.0) / noise)
                - normal(((s as f64 - OFFSET - 0.5) / amp - 1.0) / noise);
            let p0 = normal(((s as f64 - OFFSET + 0.5) / amp + 1.0) / noise)
                - normal(((s as f64 - OFFSET - 0.5) / amp + 1.0) / noise);
            metrics[0][s] = (2.0 * p0 / (p1 + p0)).log2() - bias;
            metrics[1][s] = (2.0 * p1 / (p1 + p0)).log2() - bias;
        }

        {
            let p1 = 1.0 - normal(((255.0 - OFFSET - 0.5) / amp - 1.0) / noise);
            let p0 = 1.0 - normal(((255.0 - OFFSET - 0.5) / amp + 1.0) / noise);
            metrics[0][255] = (2.0 * p0 / (p1 + p0)).log2() - bias;
            metrics[1][255] = (2.0 * p1 / (p1 + p0)).log2() - bias;
        }

        for bit in 0..2 {
            for s in 0..256 {
                self.metrics[bit][s] = match (metrics[bit][s] * scale + 0.5).floor() {
                    x if x.is_nan() => i32::MIN,
                    f64::NEG_INFINITY => i32::MIN,
                    other => other as i32,
                };
            }
        }
    }

    fn gen_table49(&mut self) {
        let mut KI: [i32; 2048] = [0; 2048];

        KI[0] = 0;
        for i in 1..2048 {
            KI[i] = (13 * KI[i - 1] + 511) % 2048;
        }

        let mut n = 0;
        for i in 0..2048 {
            if (KI[i] >= 256) && (KI[i] <= 1792) && (KI[i] != 1024) {
                self.table49[n] = KI[i] - 1024;
                n += 1;
            }
        }
    }

    pub fn frequency_deinterleave(&self, bits: &mut FicSymbol) {
        let mut result: FicSymbol = [0u8; 384].into();
        let slice = result.as_mut_bitslice();

        for n in 0..K {
            let mut k = self.table49[n as usize];

            k += match k {
                n if n < 0 => K / 2,
                n if n > 0 => K / 2 - 1,
                _ => 0,
            };
            // now, 0 <= k  < 768

            slice.set(2 * n as usize    , bits[2 * k as usize    ]);
            slice.set(2 * n as usize + 1, bits[2 * k as usize + 1]);
        }

        bits.copy_from_bitslice(slice);
    }

    pub fn qpsk_symbol_demapper(&self, bits: &mut FicSymbol) {
        let mut result: FicSymbol = [0u8; 384].into();
        let slice = result.as_mut_bitslice();

        for n in 0..K as usize {
            slice.set(n, bits[2 * n]);
            slice.set(n + K as usize, bits[(2 * n) + 1]);
        }

        bits.copy_from_bitslice(slice);
    }

    // pub fn depuncture(&self, bits: &Fic2304) -> Fic3096 {

    // }

    // pub fn viterbi(&self, bits: &Fic3096) -> Fic768 {
        
    // }
}
