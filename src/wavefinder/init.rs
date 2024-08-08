use super::Wavefinder;

const USBDATALEN: usize = 31;

fn as_u8<T>(v: Vec<T>) -> Vec<u8> {
    let (head, body, tail) = unsafe { v.align_to::<u8>() };
    assert!(head.is_empty());
    assert!(tail.is_empty());

    let mut copy = Vec::new();
    copy.extend_from_slice(body);
    copy
}

impl Wavefinder {
    /*
        The SL11R communicates with the TMS320VC5402 DSPs
        via their Host Port Interfaces (HPI)
        Looks like A1 is connected to HCNTL0 and A2 to HCNTL1
        so that the HPI registers appear at the following
        addresses in the SL11R address space:

    ** DSP A Host Port Interface (HPI) registers
    */
    /* DSP A HPI Control register */
    const HPIC_A: u16 = 0x8000;
    /* DSP A HPI Data register */
    const HPID_A: u16 = 0x8002;
    /* DSP A HPI Address register */
    const HPIA_A: u16 = 0x8004;
    /*
     ** DSP B Host Port Interface (HPI) registers
     */
    /* DSP B HPI Control register */
    const HPIC_B: u16 = 0xc110;
    /* DSP B HPI Data register */
    const HPID_B: u16 = 0xc112;
    /* DSP B HPI Address register */
    const HPIA_B: u16 = 0xc114;

    /*
     ** A Cypress Semiconductor SL11R USB microcontroller is used in the WaveFinder.
     **
     ** This file allocates symbolic names to the register addresses
     ** and so forth.
     **
     */

    /* Output Data Register 0
     **
     */
    const OUTREG0: u16 = 0xc01e;

    const OUTREG1: u16 = 0xc024;

    const IOCTRLREG1: u16 = 0xc028;

    /* The following are concerned mainly (only?) with LED control */
    /* PWM Control Register */
    const PWMCTRLREG: u16 = 0xc0e6;

    /* PWM Maximum Count Register */
    const PWMMAXCNT: u16 = 0xc0e8;

    /* PWM Channel 0 Start Register - Does this control anything ? */
    const PWMCH0STRT: u16 = 0xc0ea;

    /* PWM Channel 0 Stop Register  - Does this control anything ? */
    const PWMCH0STOP: u16 = 0xc0ec;

    /* PWM Channel 1 Start Register - Controls blue LED */
    const PWMCH1STRT: u16 = 0xc0ee;

    /* PWM Channel 1 Stop Register  - Controls blue LED*/
    const PWMCH1STOP: u16 = 0xc0f0;

    /* PWM Channel 2 Start Register - Controls red LED */
    const PWMCH2STRT: u16 = 0xc0f2;

    /* PWM Channel 2 Stop Register  - Controls red LED */
    const PWMCH2STOP: u16 = 0xc0f4;

    /* PWM Channel 3 Start Register - Controls green LED */
    const PWMCH3STRT: u16 = 0xc0f6;

    /* PWM Channel 3 Stop Register  - Controls green LED */
    const PWMCH3STOP: u16 = 0xc0f8;

    /* PWM Cycle Count Register */
    const PWMCYCCNT: u16 = 0xc0fa;

    /* DAC value - probably used by the AFC systems
     ** to offset the reference oscillator frequency
     ** Note the wavefinder firmware, as far as I can see,
     ** always writes the same value to the DAC!
     */
    const DACVALUE: u16 = 0x0366;

    /* Don't know what these do yet */
    const UNK0XC120: u16 = 0xc120;

    fn load_firmware(&self, firmware: &[u8], addrreg: u16, datareg: u16) {
        self.sendmem(0, 0, &as_u8(vec![addrreg, 0x007f, 0x0000]));

        let mut remain: usize = 0x2000 - 0x80 + 1;

        while remain > 0 {
            let mut ubuf: Vec<u8> = Vec::new();

            let datareg_bytes = datareg.to_be_bytes();
            ubuf.push(datareg_bytes[1]);
            ubuf.push(datareg_bytes[0]);

            if remain >= USBDATALEN {
                let mut j = 2;
                while j < USBDATALEN * 2 {
                    let offset = remain;
                    remain -= 1;
                    ubuf.push(firmware[0x2001 - offset]);
                    ubuf.push(0x00);
                    j += 2;
                }
                self.sendmem(datareg as u32, 0, &as_u8(ubuf));
            } else {
                let left = remain * 2;
                let mut j = 2;
                while j < (left + 2) {
                    let offset = remain;
                    remain -= 1;
                    ubuf.push(firmware[0x2000 - offset]);
                    ubuf.push(0x00);
                    j += 2;
                }
                self.sendmem(datareg as u32, 0, &as_u8(ubuf));
            }
        }
    }

    fn boot_dsps(&self) {
        let dspA = include_bytes!("rsDSPa.bin");
        let dspB = include_bytes!("rsDSPb.bin");

        self.sendmem(0, 0, &as_u8(vec![Self::HPIA_B, 0x00e0, 0x0000]));
        self.sendmem(0, 0, &as_u8(vec![Self::HPID_B, 0x0000, 0x0000]));
        self.sendmem(0, 0, &as_u8(vec![Self::HPIC_B, 0x0001, 0x0001]));
        self.sendmem(0, 0, &as_u8(vec![Self::HPIC_A, 0x0001, 0x0001]));

        self.load_firmware(dspB, Self::HPIA_B, Self::HPID_B);
        self.load_firmware(dspA, Self::HPIA_A, Self::HPID_A);

        self.sendmem(0, 0, &as_u8(vec![Self::HPIA_A, 0x007e, 0x0000]));
        self.sendmem(0, 0, &as_u8(vec![Self::HPIA_B, 0x007e, 0x0000]));
        self.sendmem(
            0,
            0,
            &as_u8(vec![Self::HPID_A, dspA[0].into(), dspA[1].into()]),
        );
        self.sendmem(
            0,
            0,
            &as_u8(vec![Self::HPID_B, dspB[0].into(), dspB[1].into()]),
        );

        self.sendmem(0, 0, &as_u8(vec![Self::HPIA_B, 0x00ff, 0x003e]));
        self.sendmem(0, 0, &as_u8(vec![Self::HPID_B, 0x0000, 0x0000]));
        self.sendmem(0, 0, &as_u8(vec![Self::HPID_B, 0x0000, 0x0000]));
        self.sendmem(0, 0, &as_u8(vec![Self::HPIA_A, 0x00ff, 0x001f]));
        self.sendmem(0, 0, &as_u8(vec![Self::HPIA_B, 0x00ff, 0x001f]));
    }

    fn timing(&self, msgnum: usize) {
        let mut timing_messages: Vec<[u8; 32]> = vec![
            [
                0x7f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x0f, 0x00,
            ],
            [
                0x7f, 0x00, 0x00, 0xfe, 0x80, 0x07, 0xe0, 0x01, 0x78, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x0f, 0x00,
            ],
            [
                0x7f, 0x00, 0x00, 0xfe, 0x80, 0x07, 0xe0, 0x01, 0x78, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x0f, 0x00,
            ],
            [
                0x7f, 0x00, 0xff, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xf8, 0xff, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x0f, 0x00,
            ],
            [
                0x7f, 0x00, 0xff, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xf8, 0xff, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x0f, 0x00,
            ],
        ];

        self.timing_msg(&mut timing_messages[msgnum]);
    }

    fn leds(&self, red: u16, blue: u16, green: u16) {
        self.mem_write(Self::PWMCH2STOP, red);
        self.mem_write(Self::PWMCH1STOP, green);
        self.mem_write(Self::PWMCH3STOP, blue);
    }

    pub fn init(&self, freq: f64) {
        self.r2_msg();
        self.mem_write(Self::PWMCTRLREG, 0);
        self.mem_write(Self::PWMMAXCNT, 0x03ff);

        self.mem_write(Self::PWMCH0STRT, 0);
        self.mem_write(Self::PWMCH0STOP, 0);

        self.mem_write(Self::PWMCH1STRT, 0);
        self.mem_write(Self::PWMCH1STOP, 0);

        self.mem_write(Self::PWMCYCCNT, 0x03ff);

        self.mem_write(Self::PWMCH2STRT, 0);
        self.mem_write(Self::PWMCH2STOP, 0);

        self.mem_write(Self::PWMCH3STRT, 0);
        self.mem_write(Self::PWMCH3STOP, 0);

        self.mem_write(Self::PWMCH0STRT, 0);
        self.mem_write(Self::PWMCH0STOP, 0x02ff);

        self.mem_write(Self::PWMCH1STOP, 0x02ff);

        self.mem_write(Self::PWMCTRLREG, 0x800f);
        self.mem_write(Self::IOCTRLREG1, 0x3de0);
        self.mem_write(Self::UNK0XC120, 0); /* TODO: work out what's at 0xc120 */
        self.sleep(100);
        self.mem_write(Self::UNK0XC120, 0xffff);
        self.mem_write(Self::OUTREG1, 0x3800); /* TODO: work out what each bit controls */
        self.mem_write(Self::OUTREG0, 0x0000);
        self.mem_write(Self::OUTREG1, 0x3000);
        self.mem_write(Self::OUTREG1, 0x3800);

        self.boot_dsps();

        self.mem_write(Self::OUTREG0, 0x1000); /* TODO: work out what each bit controls */
        self.leds(0x3ff, 0x180, 0x3ff); /* Green LED on as simple indicator */
        self.tune(freq);
        self.sleep(400);
        self.timing(0);
        self.sleep(4);
        self.timing(1);
        self.sleep(4);
        self.timing(1);
        self.sleep(4);
        self.timing(2);
        self.sleep(50);
        self.mem_write(Self::DACVALUE, 0x5330);
        self.mem_write(Self::DACVALUE, 0x5330);
        self.sleep(77);
        /* The next control message causes the WaveFinder to start sending
        isochronous data */
        self.r1_msg();
        self.mem_write(Self::PWMCTRLREG, 0x800f);
        self.timing(1);
        self.timing(2);
        self.timing(1);
        self.timing(3);
        self.tune(freq);
        self.sleep(200);
        self.timing(4);
        self.tune(freq);
        self.sleep(200);
        self.tune(freq);
        self.sleep(200);
        self.mem_write(Self::DACVALUE, 0x5330);
    }
}
