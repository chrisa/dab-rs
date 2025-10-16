use crate::{msc::MainServiceChannelFrame, output::mp2header::Mp2Header};
use std::io::{self, StdoutLock, Write};


/* Tables 20 and 21 ETSI EN 300 401 V1.3.3 (2001-05), 7.2.1.3, P.69-70
    Entries of -1 in these tables correspond to forbidden indices */
const BRTAB: [i16; 16] = [-1,32,48,56,64,80,96,112,128,160,192,224,256,320,384,-1]; 
const LBRTAB: [i16; 16] = [-1,8,16,24,32,40,48,56,64,80,96,112,128,144,160,-1];

/* These header bits are constant for DAB */
const HMASK: u32 = 0xfff70e03;
/* ..and have these values.. */
const HXOR: u32 = 0xfff40400;


pub struct Mpeg {
    header_expected: bool,
    header_valid: bool,
    stdout: StdoutLock<'static>,
}

pub fn new_mpeg() -> Mpeg {
    let stdout = io::stdout().lock();
    Mpeg { header_expected: true, header_valid: false, stdout }
}

impl Mpeg {

    pub fn output(&mut self, frame: &MainServiceChannelFrame) {
        
        if self.header_expected {
            self.header_valid = true;
            let header_bytes = frame.bits[0..4].try_into().expect("four bytes");
            let header_int = u32::from_be_bytes(header_bytes);
            let header = Mp2Header::from_u32(header_int);
            if ((header_int & HMASK) ^ HXOR) != 0 {
                eprintln!("header mask check failed: {:x}", header_int);
                self.header_valid = false;
            }
            else if header.id == false {
                self.header_expected = false;
                if LBRTAB[header.bit_rate_index as usize] != frame.bitrate as i16 {
                    eprintln!("Low bitrate conflict FIC: {} MP2 header: {}", frame.bitrate, LBRTAB[header.bit_rate_index as usize]);
                    self.header_valid = false;
                }    
            }
            else if BRTAB[header.bit_rate_index as usize] != frame.bitrate as i16 {
                    eprintln!("Bitrate conflict FIC: {} MP2 header: {}", frame.bitrate, BRTAB[header.bit_rate_index as usize]);
                    self.header_valid = false;
            }
        }
        else {
            self.header_expected = true;
        }

        if self.header_valid {
            if let Err(error) = self.stdout.write(&frame.bits) {
                panic!("failed writing to stderr: {}", error);
            }
        }

    }

}