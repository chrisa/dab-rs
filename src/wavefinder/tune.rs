use super::Wavefinder;

/* Maximum Band III frequency (MHz) */
const MAXFREQIII: f64 = 240.0;

/* For L Band, this (MHz) is subtracted from the input frequency */
const LBANDOFFSET: f64 = 1251.456;

/* The receiver Intermediate Frequency - WaveFinder uses Hitachi HWSD231 SAW filter */
const IF: f64 = 38.912e6;

/* Reference frequency */
const F_OSC: f64 = 16.384e6;

/* LMX1511 R division constant */
const R_1511: f64 = 1024.0;

/* LMX1511 Prescaler */
const P_1511: u32 = 64;

/* LMX2331A IF and RF R Counter */
const R_2331A: u32 = 256;

/* LMX2331A IF N Counter */
const NIFA_2331A: u32 = 0;
const NIFB_2331A: u32 = 40;

/* LMX2331A RF N Counter */
const NRFA_2331A: u32 = 98;
const NRFB_2331A: u32 = 152;

/* PLL Selection */
const LMX1511: u8 = 1;
const LMX2331A: u8 = 0;

/*
** Reverse the 'len' least sig bits in 'op'
*/
fn reverse_bits(op: u32, len: usize) -> u32 {
    let mut i: usize = 0;
    let mut j: u32 = 0;

    while i < len {
        if op & (1 << (len - i - 1)) != 0 {
            j |= 1 << i;
        }
        i += 1;
    }

    j
}

impl Wavefinder {
    pub fn tune(&self, freq: f64) {
        let lband;
        let offset_freq;

        if freq > MAXFREQIII {
            lband = true;
            offset_freq = freq - LBANDOFFSET;
        } else {
            lband = false;
            offset_freq = freq;
        }

        /* *Don't* change the order in which these messages are sent */
        let mut rc: u32;

        /* Load the RF R counter of the Band L PLL - constants */
        rc = 0x100000 | reverse_bits(R_2331A, 15) << 5 | 0x10;
        self.tune_msg(rc, 22, LMX2331A, lband);

        /* Load the RF N counter of the Band L PLL - constants */
        rc = 0x300000 | reverse_bits(NRFA_2331A, 7) << 13 | reverse_bits(NRFB_2331A, 11) << 2 | 2;
        self.tune_msg(rc, 22, LMX2331A, lband);

        /* Load the IF R counter of the Band L PLL - constants */
        rc = reverse_bits(R_2331A, 15) << 5 | 0x10;
        self.tune_msg(rc, 22, LMX2331A, lband);

        /* Load the N counter of the Band III PLL - this does the tuning */
        let f_vcod = (offset_freq * 1e6 + IF) / (F_OSC / R_1511);
        let f_vco = f_vcod.ceil() as u32; /* TODO: Necessary ?  Seems to be *essential* */

        /* Load the IF N counter of the Band L PLL - constants */
        rc = 0x200000 | reverse_bits(NIFA_2331A, 7) << 13 | reverse_bits(NIFB_2331A, 11) << 2 | 2;
        self.tune_msg(rc, 22, LMX2331A, lband);

        let b_1511 = f_vco / P_1511;
        let a_1511 = f_vco % P_1511;

        /* Load the R counter and S latch of the Band III PLL - constants */
        rc = 0x8000 | (reverse_bits(R_1511 as u32, 14)) << 1 | 1;
        self.tune_msg(rc, 16, LMX1511, lband);

        /* Load the N counter (as A and B counters) of the Band III PLL */
        rc = reverse_bits(a_1511, 7) << 11 | reverse_bits(b_1511, 11);
        self.tune_msg(rc, 19, LMX1511, lband);
    }
}
