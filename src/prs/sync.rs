use rustfft::num_complex::{c64, Complex64, ComplexFloat};
use rustfft::FftPlanner;

use crate::prs::reference::prs_reference_1_2;
use crate::visualiser;
use crate::visualiser::Visualiser;

use super::PhaseReferenceSymbol;

use std::f64::consts::PI;
use std::iter::zip;
use std::time::{Duration, SystemTime};

pub struct PhaseReferenceSynchroniser {
    visualiser: Visualiser,
    prs1: [Complex64; 2048],
    prs2: [Complex64; 2048],
    sync: bool,
    count: u8,
    last_cv: SystemTime,
    last_afc: SystemTime,
    ravg: RAverage,
}

pub fn new_synchroniser() -> PhaseReferenceSynchroniser {
    let vis: Visualiser =
        visualiser::create_visualiser("PRS ifft", 400, 400, -8000.0..8000.0, -8000.0..8000.0);
    let (prs1, prs2) = prs_reference_1_2();
    PhaseReferenceSynchroniser {
        visualiser: vis,
        prs1,
        prs2,
        sync: false,
        count: 3,
        last_cv: SystemTime::now(),
        last_afc: SystemTime::now(),
        ravg: RAverage { j: 0, k: 0, prev_ir: 0.0, sa: [0.0; 8] }
    }
}

fn ifft(data: &[Complex64; 2048]) -> [Complex64; 2048] {
    let mut output = [c64(0, 0); 2048];
    output.clone_from_slice(data);
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_inverse(2048);
    fft.process(&mut output);
    output
}

fn fft(data: &[Complex64; 2048]) -> [Complex64; 2048] {
    let mut output = [c64(0, 0); 2048];
    output.clone_from_slice(data);
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(2048);
    fft.process(&mut output);
    output
}

fn ref_symbol(offset: usize, source: &[Complex64; 2048]) -> [Complex64; 2080] {
    let mut symbol = [c64(0, 0); 2080];
    symbol[offset..(offset + 2048)].copy_from_slice(source);
    symbol
}

fn mpy(a: &[Complex64; 2048], b: &[Complex64; 2048], scale: f64) -> [Complex64; 2048] {
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

fn mag(data: &[Complex64; 2048]) -> [f64; 2048] {
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

fn maxext(data: &[f64; 2048]) -> (f64, i32) {
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

fn mean(data: &[f64; 2048]) -> f64 {
    let sum: f64 = data.iter().sum();
    sum / 2048.0
}

fn peak(data: &[f64; 2048], indx: i32) -> i32 {
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

fn imp(irtime: f64, mdata: &[Complex64; 2048]) -> f64
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

struct RAverage {
    j: usize,
    k: usize,
    prev_ir: f64,
    sa: [f64; 8],
}

fn raverage(r: &mut RAverage, ir: f64) -> f64
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

impl PhaseReferenceSynchroniser {
    pub fn try_sync_prs(&mut self, prs: &PhaseReferenceSymbol) -> (f64, f64) {
        let rdata = ifft(&prs.vector());
        self.visualiser.update(rdata);

        let (c, prs2_offset) = self.calc_c(&rdata);
        let ir = self.calc_ir(prs2_offset, &prs.vector());

        if (c.abs() < (2.4609375e-4/2.0)) && (ir.abs() < 350.0) {
            if self.count == 0 {
                self.sync = true;
            } else {
                self.count -= 1;
                self.sync = false;
            }
        } else {
            self.count = 3;
            self.sync = false;
        }

        let now = SystemTime::now();

        if now.duration_since(self.last_cv).unwrap() > Duration::from_millis(60) {
            //sync_cvmsg()
            self.last_cv = now;
        }

        let avg_ir = raverage(&mut self.ravg, ir);
        
        if now.duration_since(self.last_afc).unwrap() > Duration::from_millis(250) {
            //sync_afcmsg()
            self.last_afc = now;
        }

        // imsg

        (c, avg_ir)
    }

    fn calc_c(&self, rdata: &[Complex64; 2048]) -> (f64, usize)
    {
        let mut indx_n = 0i32;
        let mut indxv = 0i32;
        let mut maxv = 0.0;
        let mut c = 4.8828125e-7;

        let (count, mut prslocal) = if self.sync {
            (1_usize, ref_symbol(0, &self.prs1))
        } else {
            (25, ref_symbol(12, &self.prs1))
        };

        /* Copy 0x18 complex points from start of data and append to the end */
        for i in 0..24 {
            prslocal[2048 + i] = prslocal[i];
        }

        for i in 0..count {
            assert!(i < (2080 - 2048));
            let offset_prslocal: &[Complex64; 2048] = &prslocal[i..(2048 + i)].try_into().unwrap();
            let cdata = mpy(rdata, offset_prslocal, 1024.0);
            let mdata = fft(&cdata);
            let magdata = mag(&mdata);
            // dbg!(magdata);

            let (mut max, indx) = maxext(&magdata);
            let vmean = mean(&magdata);
            if (vmean * 12.0) > max {
                max = 0.0;
            }

            if self.sync {
                indx_n = peak(&magdata, indx);
                indx_n /= 15;

                if indx_n > 12 {
                    indx_n = 80;
                } else if indx_n < -12 {
                    indx_n = -80;
                }

                indx_n = -indx_n;
            }

            if max > maxv {
                maxv = max;
                indxv = indx;
            }
        }

        if indxv < 1024 {
            indxv = -indxv;
        } else {
            indxv = 2048 - indxv;
        }

        if self.sync {
            c *= indx_n as f64;
        } else {
            c *= indxv as f64;
        }

        (c, -indxv as usize)
    }

    fn calc_ir(&self, prs2_offset: usize, idata: &[Complex64; 2048]) -> f64
    {
        let iprslocal = ref_symbol(prs2_offset, &self.prs2);
        let mdata = mpy(idata, &iprslocal[0..2048].try_into().unwrap(), 32.0);
        let rdata = fft(&mdata);
        let magdata = mag(&rdata);
        // dbg!(magdata);

        let (mut max, indx) = maxext(&magdata);
        let vmean = mean(&magdata);
        if (vmean * 14.0) > max {
            max = 0.0;
        }

        let mut ir: f64 = indx.into();

        if ir > 1024.0 {
            ir -= 2048.0;
        }    

        let mut stf = 0.666666666;

        while (1000.0 * stf) > 2.5e-2 {
            stf /= 2.0;

            let v = ir - stf;
            let vi = imp(v, &mdata);
            if vi > max {
                max = vi;
                ir = v;
            }

            let v = ir + stf;
            let vs = imp(v, &mdata);
            if vs > max {
                max = vs;
                ir = v;
            }
        }

        ir *= 1000.0;

        ir
    }

}
