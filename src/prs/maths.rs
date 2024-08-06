use std::f64::consts::PI;
use std::iter::zip;
use rustfft::num_complex::{Complex64, ComplexFloat};

pub struct RAverage {
    j: usize,
    k: usize,
    prev_ir: f64,
    sa: [f64; 8],
}

pub fn new_raverage() -> RAverage
{
    RAverage { j: 0, k: 0, prev_ir: 0.0, sa: [0.0; 8] }
}

pub fn raverage(r: &mut RAverage, ir: f64) -> f64
{
        // int d, i;
        // double t;
    
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

    let d = if r.j != 0 {
        8
    }
    else {
        r.k
    };

    let mut t = 0.0;

    if d > 0 {
        for i in 0..d {
            t += r.sa[i];
        }
    }

    r.prev_ir = t / d as f64;
    r.prev_ir
}

pub fn mpy(a: &[Complex64; 2048], b: &[Complex64; 2048], scale: f64) -> [Complex64; 2048] {
    // for (k=0; k < n; k++) {
    // 	*(dst + k) = (*(srca + k) * *(srcb + k));
    // 	*(dst + k) = *(dst + k)/1024;
    // }

    // let mut result = [c64(0, 0); 2048];
    // for i in 0..2048 {
    //     result[i] = (a[i] * b[b_offset + i]) / 1024;
    // }
    // result

    zip(a, b)
    .map(|(v1, v2)| (v1 * v2) / scale)
    .collect::<Vec<Complex64>>()
    .try_into()
    .unwrap()
}

pub fn mag(data: &[Complex64; 2048]) -> [f64; 2048] {
    // for (i=0; i < n; i++)
    //   *(out+i) = cabs(*(in+i))/n;

    // let mut result = [0.0; 2048];
    // for i in 0..2048 {
    //     result[i] = data[i].abs() / 2048;
    // }
    // result

    data.map(|c| c.abs() / 2048.0)
}

// double maxext(double *in, int n, int *index)
// {
// 	int i;
// 	double max;

// 	max = *in;
// 	*index = 0;

// 	for (i=1; i < n; i++)
// 		if (*(in+i) > max) {
// 			max = *(in+i);
// 			*index = i;
// 		}

// 	return(max);
// }

pub fn maxext(data: &[f64; 2048]) -> (f64, i32) {
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

// double mean(double *in, int n)
// {
// 	int i;

// 	double out = 0.0L;

// 	for (i=0; i < n; i++)
// 		out += *(in+i);

// 	out = out/n;

// 	return(out);
// }

pub fn mean(data: &[f64; 2048]) -> f64 {
    let sum: f64 = data.iter().sum();
    sum / 2048.0
}

pub fn peak(data: &[f64; 2048], indx: i32) -> i32 {
    // 	double b, c, d, bmax = 0.0;
    // 	int a, i, l;
    // 	int pts = 0x800;
    // 	int res = 0;

    // 	a = indx - 0x1f8 + pts;
    // 	b = 0x1f8 / 2;
    // 	l = a + b - 1;

    // 	for (i = 0; i < (2 * 0x1f8); i++) {
    // 		c = *(magdata + ((pts - 0x1f8 / 2 + a) % 0x800));
    // 		d = *(magdata + (l % 0x800));
    // 		b = b + d - c;
    // 		if (b > bmax) {
    // 			bmax = b;
    // 			res = a % 0x800;
    // 		}
    // 		l++;
    // 		a++;
    // 	}

    // 	if (res > 0x400)
    // 		res = res - 0x800;

    // 	return (res);
    // }
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

pub fn imp(irtime: f64, mdata: &[Complex64; 2048]) -> f64
{
	let ir = irtime * 4096.0;
	let jr = ir as i32 & 0x7fffff;

    let mut a = 2048;
    let mut m = a;
    let mut d: i32;
    let mut j = 0.0;
    let mut k = 0.0;
    
    for md in mdata.iter().take(2048) {
		a = m;
		m = (m as i32).wrapping_add(jr); // !!!
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

	let ri = ((k*k) + (j*j)).sqrt();
	let a1c = 0x800 * 32767;

	1.0 / a1c as f64 * ri
}
