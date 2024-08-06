use rustfft::num_complex::{c64, Complex64};

use crate::prs::reference::prs_reference_1_2;
use crate::visualiser;
use crate::visualiser::Visualiser;
use crate::prs::maths::*;
use crate::prs::fft::*;

use super::PhaseReferenceSymbol;

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
        ravg: new_raverage(),
    }
}



fn ref_symbol(offset: usize, source: &[Complex64; 2048]) -> [Complex64; 2080] {
    let mut symbol = [c64(0, 0); 2080];
    symbol[offset..(offset + 2048)].copy_from_slice(source);
    symbol
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
