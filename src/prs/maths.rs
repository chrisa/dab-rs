use crate::prs::PhaseReferenceArray;
use rustfft::num_complex::{Complex64, ComplexFloat};
use std::f64::consts::PI;
use std::iter::zip;

use super::PRS_POINTS;

pub struct RAverage {
    j: usize,
    k: usize,
    prev_ir: f64,
    sa: [f64; 8],
}

pub fn new_raverage() -> RAverage {
    RAverage {
        j: 0,
        k: 0,
        prev_ir: 0.0,
        sa: [0.0; 8],
    }
}

pub fn raverage(r: &mut RAverage, ir: f64) -> f64 {
    if r.prev_ir.abs() > 350.0 {
        r.k = 0;
        r.j = 0;
    }

    r.sa[r.k] = ir;
    r.k += 1;

    if r.k == 8 {
        r.k = 0;
        r.j = 1;
    }

    let d = if r.j != 0 { 8 } else { r.k };

    let mut t = 0.0;

    if d > 0 {
        for i in 0..d {
            t += r.sa[i];
        }
    }

    r.prev_ir = t / d as f64;
    r.prev_ir
}

pub fn mpy(a: &PhaseReferenceArray, b: &PhaseReferenceArray, scale: f64) -> PhaseReferenceArray {
    zip(a, b)
        .map(|(v1, v2)| (v1 * v2) / scale)
        .collect::<Vec<Complex64>>()
        .try_into()
        .unwrap()
}

pub fn mag(data: &PhaseReferenceArray) -> [f64; PRS_POINTS] {
    data.map(|c| c.abs() / PRS_POINTS as f64)
}

pub fn maxext(data: &[f64; PRS_POINTS]) -> (f64, i32) {
    let mut index = 0i32;
    let mut max = 0.0;

    for (i, val) in data.iter().enumerate() {
        if *val > max {
            max = *val;
            index = i as i32;
        }
    }

    (max, index)
}

pub fn mean(data: &[f64; PRS_POINTS]) -> f64 {
    let sum: f64 = data.iter().sum();
    sum / PRS_POINTS as f64
}

pub fn peak(data: &[f64; PRS_POINTS], indx: i32) -> i32 {
    let mut a = indx - 504 + 2048;
    let mut b: f64 = 504.0 / 2.0;
    let mut l = a + (504 / 2) - 1;
    let mut bmax = 0.0;
    let mut res: i32 = 0;

    for _ in 0..(2 * 504) {
        let c_index = (2048 - 504 / 2 + a) % 2048;
        let c = data[c_index as usize];
        let d_index = l % 2048;
        let d = data[d_index as usize];
        b = b + d - c;
        if b > bmax {
            bmax = b;
            res = a % 2048;
        }
        l += 1;
        a += 1;
    }

    if res > 1024 {
        res -= 2048;
    }

    res
}

pub fn imp(irtime: f64, mdata: &PhaseReferenceArray) -> f64 {
    let ir = irtime * 4096.0;
    let jr = ir as i32 & 0x7fffff;

    let mut a = 2048;
    let mut m: i32 = a;
    let mut d: i32;
    let mut j = 0.0;
    let mut k = 0.0;

    for md in mdata.iter().take(2048) {
        a = m;
        m = m.wrapping_add(jr); // !!!
        a >>= 12;
        d = a & 0x7ff;
        a -= 512;
        a &= 0x7ff;

        /* Too slow! */
        let cosa = (1 << 15) as f64 * (a as f64 * 2.0 * PI / 2048.0).cos();
        let cosd = (1 << 15) as f64 * (d as f64 * 2.0 * PI / 2048.0).cos();

        // cosa = *(sync->refs->cos_table + a);
        // cosd = *(sync->refs->cos_table + d);

        let re_prs = md.re;
        let im_prs = md.im;
        let p = im_prs * cosd;
        let q = im_prs * cosa;
        let mut r = re_prs * cosa;
        let mut s = re_prs * cosd;
        s -= q;
        r += p;
        k += s;
        j += r;
    }

    let ri = ((k * k) + (j * j)).sqrt();
    let a1c = 0x800 * 32767;

    1.0 / a1c as f64 * ri
}
