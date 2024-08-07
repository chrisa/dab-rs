use rustfft::num_complex::c64;
use rustfft::num_complex::Complex64;

use crate::prs::fft::*;
use crate::prs::maths::*;
use crate::prs::reference::prs_reference_1_2;
use crate::prs::PhaseReferenceArray;
use crate::prs::PhaseReferenceSymbol;
use crate::prs::PRS_POINTS;
use crate::visualiser;
use crate::visualiser::Visualiser;
use crate::wavefinder::timing_msg;
use crate::wavefinder::Message;

use std::time::{Duration, SystemTime};

pub struct PhaseReferenceSynchroniser {
    visualiser: Visualiser,
    prs1: PhaseReferenceArray,
    prs2: PhaseReferenceArray,
    sync: bool,
    count: u8,
    last_cv: SystemTime,
    last_afc: SystemTime,
    ravg: RAverage,
    selstr: [u8; 10],
}

pub fn new_synchroniser() -> PhaseReferenceSynchroniser {
    let vis: Visualiser =
        visualiser::create_visualiser("PRS ifft", 400, 400, -80000.0..80000.0, -80000.0..80000.0);
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
        selstr: [0; 10],
    }
}

fn align_reference_symbol(indx: i32, source: &PhaseReferenceArray) -> [Complex64; 2080] {
    let mut symbol = [c64(0, 0); 2080];
    if indx == 0 {
        symbol[0..PRS_POINTS].copy_from_slice(source);
        return symbol;
    }
    let offset = indx.unsigned_abs() as usize;
    assert!(offset <= PRS_POINTS);
    if indx > 0 {
        symbol[0..offset].copy_from_slice(&source[(PRS_POINTS - offset)..PRS_POINTS]);
        symbol[offset..PRS_POINTS].copy_from_slice(&source[0..(PRS_POINTS - offset)]);
    }
    if indx < 0 {
        symbol[(PRS_POINTS - offset)..PRS_POINTS].copy_from_slice(&source[0..offset]);
        symbol[0..(PRS_POINTS - offset)].copy_from_slice(&source[offset..PRS_POINTS]);
    }
    symbol
}

impl PhaseReferenceSynchroniser {
    pub fn try_sync_prs(&mut self, prs: &PhaseReferenceSymbol) -> Vec<Message> {
        let rdata = ifft(&prs.vector());
        let (c, prs2_offset) = self.calc_c(&rdata);
        let ir = self.calc_ir(prs2_offset, &prs.vector());

        if (c.abs() < (2.4609375e-4 / 2.0)) && (ir.abs() < 350.0) {
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

        let mut messages: Vec<Message> = Vec::new();

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

        messages.push(self.sync_imsg(avg_ir));

        dbg!(c, avg_ir);

        messages
    }

    fn calc_c(&mut self, rdata: &PhaseReferenceArray) -> (f64, i32) {
        let mut indx_n = 0i32;
        let mut indxv = 0i32;
        let mut maxv = 0.0;
        let mut c = 4.8828125e-7;

        let (count, mut prslocal) = if self.sync {
            (1_usize, align_reference_symbol(0, &self.prs1))
        } else {
            (25, align_reference_symbol(12, &self.prs1))
        };

        /* Copy 0x18 complex points from start of data and append to the end */
        for i in 0..24 {
            prslocal[PRS_POINTS + i] = prslocal[i];
        }

        for i in 0..count {
            assert!(i < (2080 - PRS_POINTS));
            let offset_prslocal: &PhaseReferenceArray =
                &prslocal[i..(PRS_POINTS + i)].try_into().unwrap();
            let cdata = mpy(rdata, offset_prslocal, 1024.0);
            self.visualiser.update(cdata);
            let mdata = fft(&cdata);
            let magdata = mag(&mdata);

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

        (c, -indxv)
    }

    fn calc_ir(&mut self, prs2_offset: i32, idata: &PhaseReferenceArray) -> f64 {
        let iprslocal = align_reference_symbol(prs2_offset, &self.prs2);
        let mdata = mpy(idata, &iprslocal[0..PRS_POINTS].try_into().unwrap(), 32.0);
        let rdata = fft(&mdata);
        let magdata = mag(&rdata);

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

    fn sync_imsg(&mut self, ir: f64) -> Message {
        const chgstr: [u8; 10] = [0x00, 0xf0, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11];

        let w1: i16 = (ir * 81.66400146484375) as i32 as i16;
        let w2: i16 = (ir * 1.024) as i32 as i16;

        let mut symstr: [u8; 10] = [0; 10];

        if self.count > 0 {
            symstr.copy_from_slice(&chgstr);
            self.count -= 1;
        } else {
            symstr.copy_from_slice(&self.selstr);
        }

        let mut imsg: [u8; 32] = [
            0x7f, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x0f, 0x00,
        ];

        imsg[2..12].copy_from_slice(&symstr);

        let w1_bytes = w1.to_be_bytes();
        let w2_bytes = w2.to_be_bytes();
        imsg[24] = w1_bytes[1];
        imsg[25] = w1_bytes[0];
        imsg[26] = w2_bytes[1];
        imsg[27] = w2_bytes[0];

        timing_msg(&imsg)
    }
}
