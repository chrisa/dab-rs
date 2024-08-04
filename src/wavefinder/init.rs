use super::Wavefinder;
use crate::wavefinder::*;

const USBDATALEN: usize = 31;

fn as_u8<u16>(v: Vec<u16>) -> Vec<u8>
{
    let (head, body, tail) = unsafe { v.align_to::<u8>() };
    assert!(head.is_empty());
    assert!(tail.is_empty());

    let mut copy = Vec::new();
    copy.extend_from_slice(body);
    copy
}

impl Wavefinder {

    fn load_firmware(&self, firmware: &[u8], addrreg: u16, datareg: u16)
    {
        self.sendmem(0, 0, &mut as_u8(vec!(addrreg, 0x007f, 0x0000)));

        let mut remain: usize = 0x2000 - 0x80 + 1;

        while remain > 0 {
            let mut ubuf: Vec<u8> = Vec::new();

            let datareg_bytes = datareg.to_be_bytes();
            ubuf.push(datareg_bytes[1]);
            ubuf.push(datareg_bytes[0]);

            if remain >= USBDATALEN {
                let mut j = 2;
                while j < USBDATALEN*2 {
                    let offset = remain;
                    remain -= 1;
                    ubuf.push(firmware[0x2001 - offset]);
                    ubuf.push(0x00);
                    j += 2;
                }
                self.sendmem(datareg as u32, 0, &mut as_u8(ubuf));
            }
            else {
                let left = remain * 2;
                let mut j = 2;
                while j < (left + 2) {
                    let offset = remain;
                    remain -= 1;
                    ubuf.push(firmware[0x2000 - offset]);
                    ubuf.push(0x00);
                    j += 2;
                }
                self.sendmem(datareg as u32, 0, &mut as_u8(ubuf));
            }
        }
        
    }



    fn boot_dsps(&self)
    {
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
        
        let dspA = include_bytes!("rsDSPa.bin");
        let dspB = include_bytes!("rsDSPb.bin");

        self.sendmem(0, 0, &mut as_u8(vec!(HPIA_B, 0x00e0, 0x0000)));
        self.sendmem(0, 0, &mut as_u8(vec!(HPID_B, 0x0000, 0x0000)));
        self.sendmem(0, 0, &mut as_u8(vec!(HPIC_B, 0x0001, 0x0001)));
        self.sendmem(0, 0, &mut as_u8(vec!(HPIC_A, 0x0001, 0x0001)));

        self.load_firmware(dspB, HPIA_B, HPID_B);
        self.load_firmware(dspA, HPIA_A, HPID_A);

        self.sendmem(0, 0, &mut as_u8(vec!(HPIA_A, 0x007e, 0x0000)));
        self.sendmem(0, 0, &mut as_u8(vec!(HPIA_B, 0x007e, 0x0000)));
        self.sendmem(0, 0, &mut as_u8(vec!(HPID_A, dspA[0].into(), dspA[1].into())));
        self.sendmem(0, 0, &mut as_u8(vec!(HPID_A, dspA[0].into(), dspA[1].into())));
        self.sendmem(0, 0, &mut as_u8(vec!(HPID_B, dspB[0].into(), dspB[1].into())));
        self.sendmem(0, 0, &mut as_u8(vec!(HPID_B, dspB[0].into(), dspB[1].into())));

        self.sendmem(0, 0, &mut as_u8(vec!(HPIA_B, 0x00ff, 0x003e)));
        self.sendmem(0, 0, &mut as_u8(vec!(HPID_B, 0x0000, 0x0000)));
        self.sendmem(0, 0, &mut as_u8(vec!(HPID_B, 0x0000, 0x0000)));
        self.sendmem(0, 0, &mut as_u8(vec!(HPIA_A, 0x00ff, 0x001f)));
        self.sendmem(0, 0, &mut as_u8(vec!(HPIA_B, 0x00ff, 0x001f)));
    }

    fn req1_req2(&self, reqnum: u32, _msgnum: u32) {
        let mut r2: [u8; 64] = [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
                        0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
                        0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
                        0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
                        0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
                        0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
                        0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
                        0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00];
      	if reqnum == 1 {
	    	self.r1_msg(&mut r2);
        }
    	else {
	    	self.r2_msg(&mut r2);
        }
    }

    fn timing(&self, msgnum: usize)
    {
        let mut timing_messages: Vec<[u8; 32]> = vec!(
            [0x7f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x00],
            [0x7f, 0x00, 0x00, 0xfe, 0x80, 0x07, 0xe0, 0x01,
            0x78, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x00],
            [0x7f, 0x00, 0x00, 0xfe, 0x80, 0x07, 0xe0, 0x01,
            0x78, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x00],
            [0x7f, 0x00, 0xff, 0x7f, 0xff, 0xff, 0xff, 0xff,
            0xf8, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x00],
            [0x7f, 0x00, 0xff, 0x7f, 0xff, 0xff, 0xff, 0xff,
            0xf8, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x00]
        );

        self.timing_msg(&mut timing_messages[msgnum]);
    }

    fn leds(&self, red: u16, blue: u16, green: u16)
    {
		self.mem_write(PWMCH2STOP, red);
		self.mem_write(PWMCH1STOP, green);
		self.mem_write(PWMCH3STOP, blue);
    }

    pub fn init(&self, freq: f64) {
        self.req1_req2(2, 0);
        self.mem_write(PWMCTRLREG, 0);
        self.mem_write(PWMMAXCNT, 0x03ff);
    
        self.mem_write(PWMCH0STRT, 0);
        self.mem_write(PWMCH0STOP, 0);
    
        self.mem_write(PWMCH1STRT, 0);
        self.mem_write(PWMCH1STOP, 0);
    
        self.mem_write(PWMCYCCNT, 0x03ff);
    
        self.mem_write(PWMCH2STRT, 0);
        self.mem_write(PWMCH2STOP, 0);
    
        self.mem_write(PWMCH3STRT, 0);
        self.mem_write(PWMCH3STOP, 0);
    
        self.mem_write(PWMCH0STRT, 0);
        self.mem_write(PWMCH0STOP, 0x02ff);
    
        self.mem_write(PWMCH1STOP, 0x02ff);
    
        self.mem_write(PWMCTRLREG, 0x800f);
        self.mem_write(IOCTRLREG1, 0x3de0);
        self.mem_write(UNK0XC120, 0);        /* TODO: work out what's at 0xc120 */
        self.sleep(100);
        self.mem_write(UNK0XC120, 0xffff);
        self.mem_write(OUTREG1, 0x3800);     /* TODO: work out what each bit controls */
        self.mem_write(OUTREG0, 0x0000);
        self.mem_write(OUTREG1, 0x3000);
        self.mem_write(OUTREG1, 0x3800);

        self.boot_dsps();

        self.mem_write(OUTREG0, 0x1000);     /* TODO: work out what each bit controls */
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
        self.mem_write(DACVALUE, 0x5330);
        self.mem_write(DACVALUE, 0x5330);
        self.sleep(77);
        /* The next control message causes the WaveFinder to start sending
           isochronous data */
        self.req1_req2(1, 1);
        self.mem_write(PWMCTRLREG, 0x800f);
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
        self.mem_write(DACVALUE, 0x5330);
    }

}