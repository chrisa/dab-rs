use crate::{msc::MainServiceChannelFrame, output::mp2header::Mp2Header};
use alsa::pcm::{Access, Format, HwParams, PCM};
use alsa::{Direction, ValueOr};

use symphonia::core::audio::{AudioBuffer, AudioBufferRef, Signal};
use symphonia::core::codecs::{CODEC_TYPE_MP2, Decoder, DecoderOptions};
use symphonia::core::formats::Packet;
use symphonia::default::get_codecs;

/* Tables 20 and 21 ETSI EN 300 401 V1.3.3 (2001-05), 7.2.1.3, P.69-70
Entries of -1 in these tables correspond to forbidden indices */
const BRTAB: [i16; 16] = [
    -1, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, -1,
];
const LBRTAB: [i16; 16] = [
    -1, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, -1,
];

/* These header bits are constant for DAB */
const HMASK: u32 = 0xfff70e03;
/* ..and have these values.. */
const HXOR: u32 = 0xfff40400;

pub struct Mpeg {
    header_expected: bool,
    header_valid: bool,
    pcm: PCM,
    decoder: Box<dyn Decoder>,
}

pub fn new_mpeg() -> Mpeg {
    // Open default playback device
    let pcm = PCM::new("default", Direction::Playback, false).unwrap();

    let mut codec_params = symphonia::core::codecs::CodecParameters::new();
    codec_params.codec = CODEC_TYPE_MP2;

    let decoder = get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .expect("decoder");

    Mpeg {
        header_expected: true,
        header_valid: false,
        pcm,
        decoder,
    }
}

/// Interleave a planar f32 AudioBuffer into Vec<f32> (frames x channels).
fn interleave_planar_f32(buf: &AudioBuffer<f32>) -> Vec<f32> {
    let channels = buf.spec().channels.count();
    let frames = buf.frames();
    let mut out = Vec::with_capacity(channels * frames);
    for f in 0..frames {
        for ch in 0..channels {
            out.push(buf.chan(ch)[f]);
        }
    }
    out
}

impl Mpeg {
    pub fn init(&mut self) {
        // Set hardware parameters: 48000 Hz / Mono / 16 bit
        let hwp = HwParams::any(&self.pcm).unwrap();
        hwp.set_channels(2).unwrap();
        hwp.set_rate(48000, ValueOr::Nearest).unwrap();
        hwp.set_format(Format::FloatLE).unwrap();
        hwp.set_access(Access::RWInterleaved).unwrap();
        self.pcm.hw_params(&hwp).unwrap();

        // Make sure we don't start the stream too early
        let hwp = self.pcm.hw_params_current().unwrap();
        let swp = self.pcm.sw_params_current().unwrap();
        swp.set_start_threshold(hwp.get_buffer_size().unwrap())
            .unwrap();
        self.pcm.sw_params(&swp).unwrap();
    }

    pub fn output(&mut self, frame: &MainServiceChannelFrame) {
        if self.header_expected {
            self.header_valid = true;
            let header_bytes = frame.bits[0..4].try_into().expect("four bytes");
            let header_int = u32::from_be_bytes(header_bytes);
            let header = Mp2Header::from_u32(header_int);
            if ((header_int & HMASK) ^ HXOR) != 0 {
                // eprintln!("header mask check failed: {:x}", header_int);
                self.header_valid = false;
            } else if !header.id {
                self.header_expected = false;
                if LBRTAB[header.bit_rate_index as usize] != frame.bitrate as i16 {
                    // eprintln!(
                    //     "Low bitrate conflict FIC: {} MP2 header: {}",
                    //     frame.bitrate, LBRTAB[header.bit_rate_index as usize]
                    // );
                    self.header_valid = false;
                }
            } else if BRTAB[header.bit_rate_index as usize] != frame.bitrate as i16 {
                // eprintln!(
                //     "Bitrate conflict FIC: {} MP2 header: {}",
                //     frame.bitrate, BRTAB[header.bit_rate_index as usize]
                // );
                self.header_valid = false;
            }
        } else {
            self.header_expected = true;
        }

        if self.header_valid {
            // Wrap your frame bytes in a Packet
            let packet = Packet::new_from_slice(0, 0, 0, &frame.bits);

            // Decode the packet.
            match self.decoder.decode(&packet) {
                Ok(audio_ref) => {
                    match audio_ref {
                        AudioBufferRef::F32(buf) => {
                            // Write to ALSA (interleave first).
                            if let Ok(io) = &self.pcm.io_f32() {
                                let interleaved = interleave_planar_f32(&buf);
                                io.writei(&interleaved).expect("writing to audio device");
                            } else {
                                panic!("getting io from pcm");
                            }
                        }
                        _ => panic!("unexpected audio format"),
                    }
                }
                Err(_) => {
                    // eprintln!("Frame decode error: {:?}", e);
                    // Continue on decode errors (or break depending on your use case).
                }
            }
        }
    }
}
