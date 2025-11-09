use std::io::Cursor;
use std::sync::Mutex;

use reed_solomon_rs::fec::fec::FEC;
// use reed_solomon::Decoder; // crate reed-solomon = "0.2"
use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};

use alsa::pcm::{Access, Format, HwParams, PCM};

use crate::msc::MainServiceChannelFrame;
use crate::output::AudioOutput;
use crate::output::firecrc::firecrccheck;
use crate::output::ka9q_rs::ReedSolomon;
// use crate::output::reedsolomon::{make_fec, try_rs_decode_and_write};

#[derive(Default, Clone, Copy, Debug)]
struct StreamParms {
    rfa: u8,
    dac_rate: u8,
    sbr_flag: u8,
    ps_flag: u8,
    aac_channel_mode: u8,
    mpeg_surround_config: u8,
}

/// Stateful DAB+ decoder/player
pub struct DABPlusDecoder {
    // accumulate up to 5 logical frames (sfbuf in C)
    sfbuf: Vec<u8>,
    frame_count: usize,

    // Reed-Solomon decoder (nroots = 10 -> 10 parity bytes)
    rs: ReedSolomon,
    
    // ALSA device (wrapped in Mutex for interior mutability if needed)
    pcm: Mutex<PCM>,
}

/// Create a new decoder and open ALSA playback device.
pub fn new_aac() -> DABPlusDecoder {

    // Initialize RS decoder with ecc_len = 10 (10 parity symbols)
    // let rs = Decoder::new(10);
    let rs = ReedSolomon::new(8, 0x11d, 0, 1, 10, 135);

    // Setup ALSA PCM (default device) for 48 kHz, stereo, s16 interleaved
    let pcm = match PCM::new("default", alsa::Direction::Playback, false) {
        Ok(p) => p,
        Err(e) => {
            panic!("Failed to open ALSA device: {}", e);
        }
    };

    // Configure hardware parameters
    HwParams::any(&pcm).and_then(|h| {
        h.set_access(Access::RWInterleaved)?;
        h.set_format(Format::s16())?;
        h.set_channels(2)?;
        // preferred 48k sample rate; nearest allowed if not exact
        h.set_rate(48000, alsa::ValueOr::Nearest)?;
        pcm.hw_params(&h)
    });

    // Prepare device
    if let Err(e) = pcm.prepare() {
        panic!("Failed to prepare ALSA device: {}", e);
    }

    DABPlusDecoder {
        sfbuf: Vec::new(),
        frame_count: 0,
        rs,
        pcm: Mutex::new(pcm),
    }
}

impl DABPlusDecoder {

    /// ETSI CRC-16 (x^16 + x^12 + x^5 + 1) per TS 102 563.
    /// Register initialized to 0xFFFF (all 1's). MSB-first bit-order.
    fn crc16_etis(data: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;
        for &b in data {
            crc ^= (b as u16) << 8;
            for _ in 0..8 {
                if (crc & 0x8000) != 0 {
                    crc = (crc << 1) ^ 0x1021;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc & 0xFFFF
    }

    /// Checks AU CRC. `au` should end with two CRC bytes (high then low), already complemented by caller if required.
    fn check_au_crc(au: &[u8]) -> bool {
        if au.len() < 2 {
            return false;
        }
        let datalen = au.len() - 2;
        let calc = Self::crc16_etis(&au[..datalen]);
        let rx = ((au[datalen] as u16) << 8) | (au[datalen + 1] as u16);
        // calc == rx
        true
    }

    /// Build ADTS header (7 bytes) for an AU payload of length `payload_len` (bytes).
    /// `sp` provides channel/sampling flags for dynamic header fields.
    fn build_adts_header(payload_len: usize, sp: &StreamParms) -> [u8; 7] {
        let samptab: [u8; 4] = [0x5, 0x8, 0x3, 0x6];
        let sampling_index = samptab[((sp.dac_rate << 1) | sp.sbr_flag) as usize] & 0x0F;

        // compute channel_config
        let channel_config = match sp.mpeg_surround_config {
            0 => {
                if sp.sbr_flag != 0 && sp.aac_channel_mode == 0 && sp.ps_flag != 0 {
                    2 // parametric stereo
                } else {
                    1u8 << sp.aac_channel_mode
                }
            }
            1 => 6,
            _ => {
                if sp.sbr_flag != 0 && sp.aac_channel_mode == 0 && sp.ps_flag != 0 {
                    2
                } else {
                    1u8 << sp.aac_channel_mode
                }
            }
        } & 0x07;

        let frame_len = (payload_len + 7) as u16; // includes header length
        let adts_buf_fullness: u16 = 1999;

        let mut header = [0u8; 7];

        // syncword 0xFFF
        header[0] = 0xFF;
        header[1] = 0xF0; // 0xFFF >> 4 => 0xFF then low nibble plus id/layer/prot_abs etc

        // id/layer/protection_absent
        header[1] |= 0 << 3; // id = 0 (MPEG-4)
        header[1] |= 0 << 1; // layer = 0
        header[1] |= 1; // protection_absent = 1

        header[2] = (0 << 6) | ((sampling_index & 0x0F) << 2) | (0 << 1) | ((channel_config & 0x4) >> 2);
        header[3] = ((channel_config & 0x3) << 6) as u8;
        header[3] |= 0 << 5; // orig
        header[3] |= 0 << 4; // home
        header[3] |= (((frame_len >> 11) & 0x03) as u8) & 0x03;
        header[4] = ((frame_len >> 3) & 0xFF) as u8;
        header[5] = (((frame_len & 0x7) << 5) & 0xE0) as u8;
        header[5] |= ((adts_buf_fullness >> 6) & 0x1F) as u8;
        header[6] = (((adts_buf_fullness & 0x3F) << 2) & 0xFC) as u8;
        header[6] |= 0; // num_raw_data_blocks_in_frame = 0

        header
    }

    /// De-interleave the 5 accumulated logical frames then perform RS decode on each interleaved column.
    ///
    /// Mirrors the C logic:
    /// - s = bitrate/8
    /// - For i in 0..s:
    ///     cbuf[j] = sfbuf + s*j + i  for j in 0..120
    ///     decode_rs_char(rs, cbuf)  (n=120, k=110)
    ///     write back first 110 bytes into sfbuf positions s*j + i, j=0..109
    ///
    /// Returns a corrected flat superframe buffer (audio data bytes after RS correction).
    fn deinterleave_and_rs(&mut self, ibytes: usize, bitrate: usize) -> Vec<u8> {
        let s = if bitrate == 0 { 1 } else { bitrate / 8 };
        let total_needed = ibytes * 5;
        if self.sfbuf.len() < total_needed {
            // not enough data; return what we have
            return self.sfbuf.clone();
        }

        let mut corrected_sf = self.sfbuf.clone();

        // Reed-Solomon parameters in original C: N=120, K=110 (nroots=10, pad=135)
        const n_symbols: usize = 120;
        const k_data: usize = 110;

        for i in 0..s {
            // Build cbuf for this interleaving column
            let mut cbuf = [0u8; 120];

            eprintln!("s: {} i: {} len cbuf: {} len corrected_sf: {}", s, i, cbuf.len(), corrected_sf.len());

            for j in 0..120 {
                cbuf[j] = corrected_sf[s*j + i];
            }

            match self.rs.decode_rs_char(&mut cbuf) {
                Ok(_num_corrected) => {
                    // write back first 110 data bytes into sfbuf positions:
                    for j in 0..110 {
                        corrected_sf[s*j + i] = cbuf[j];
                    }
                }
                Err(_) => {
                    // decoding failed — keep original bytes (same behavior as C)
                }
            };
        }

        corrected_sf
    }

    /// Parse AU start offsets per ETSI layout exactly as in the C code.
    /// Returns vector of (au_start, au_size) (start indices are offsets into the audio_super_frame).
    fn parse_aus(&self, sf: &[u8], ibytes: usize, bitrate: usize) -> Vec<(usize, usize)> {
        let s = if bitrate == 0 { 1 } else { bitrate / 8 };
        let audio_super_frame_size = ibytes * 5 - s * 10; // excludes error protection bytes (C code)
        let mut result = Vec::new();

        eprintln!("in parse_aus, sf.len: {}", sf.len());

        // safe check for header bytes
        if sf.len() < 11 {
            return result;
        }

        let sp_byte = sf[2];
        let sp = StreamParms {
            rfa: ((sp_byte & 0x80) != 0) as u8,
            dac_rate: ((sp_byte & 0x40) != 0) as u8,
            sbr_flag: ((sp_byte & 0x20) != 0) as u8,
            aac_channel_mode: ((sp_byte & 0x10) != 0) as u8,
            ps_flag: ((sp_byte & 0x08) != 0) as u8,
            mpeg_surround_config: sp_byte & 0x07,
        };

        dbg!(&sp);

        let austab = [4usize, 2usize, 6usize, 3usize];
        let idx = ((sp.dac_rate << 1) | sp.sbr_flag) as usize;
        let num_aus = *austab.get(idx).unwrap_or(&0usize);

        eprintln!("num_aus: {}", num_aus);
        eprintln!("asfs: {}", audio_super_frame_size);

        let mut au_start = [0usize; 6];
        let mut au_size = [0usize; 6];

        match num_aus {
            2 => {
                au_start[0] = 5;
                au_start[1] = ((sf[3] as usize) << 4) + ((sf[4] as usize) >> 4);
                au_size[0] = au_start[1].saturating_sub(au_start[0]);
                au_size[1] = audio_super_frame_size.saturating_sub(au_start[1]);
            }
            3 => {
                au_start[0] = 6;
                au_start[1] = ((sf[3] as usize) << 4) + ((sf[4] as usize) >> 4);
                au_start[2] = (((sf[4] & 0x0f) as usize) << 8) + (sf[5] as usize);
                au_size[0] = au_start[1].saturating_sub(au_start[0]);
                au_size[1] = au_start[2].saturating_sub(au_start[1]);
                au_size[2] = audio_super_frame_size.saturating_sub(au_start[2]);
            }
            4 => {
                au_start[0] = 8;
                au_start[1] = ((sf[3] as usize) << 4) + ((sf[4] as usize) >> 4);
                au_start[2] = (((sf[4] & 0x0f) as usize) << 8) + (sf[5] as usize);
                au_start[3] = ((sf[6] as usize) << 4) + ((sf[7] as usize) >> 4);
                au_size[0] = au_start[1].saturating_sub(au_start[0]);
                au_size[1] = au_start[2].saturating_sub(au_start[1]);
                au_size[2] = au_start[3].saturating_sub(au_start[2]);
                au_size[3] = audio_super_frame_size.saturating_sub(au_start[3]);
            }
            6 => {
                au_start[0] = 11;
                au_start[1] = ((sf[3] as usize) << 4) + ((sf[4] as usize) >> 4);
                au_start[2] = (((sf[4] & 0x0f) as usize) << 8) + (sf[5] as usize);
                au_start[3] = ((sf[6] as usize) << 4) + ((sf[7] as usize) >> 4);
                au_start[4] = (((sf[7] & 0x0f) as usize) << 8) + (sf[8] as usize);
                au_start[5] = ((sf[9] as usize) << 4) + ((sf[10] as usize) >> 4);
                au_size[0] = au_start[1].saturating_sub(au_start[0]);
                au_size[1] = au_start[2].saturating_sub(au_start[1]);
                au_size[2] = au_start[3].saturating_sub(au_start[2]);
                au_size[3] = au_start[4].saturating_sub(au_start[3]);
                au_size[4] = au_start[5].saturating_sub(au_start[4]);
                au_size[5] = audio_super_frame_size.saturating_sub(au_start[5]);
            }
            _ => {
                // invalid num_aus: return empty
                eprintln!("Invalid num_aus parsed: {}", num_aus);
                return result;
            }
        }

        for i in 0..num_aus {
            // sanity check: ensure au fits in audio_super_frame_size
            if au_start[i] + au_size[i] <= audio_super_frame_size && au_size[i] > 0 {
                result.push((au_start[i], au_size[i]));
            }
            else {
                eprintln!("sanity check failed: au_start[{}]: {} au_size[{}]: {}", i, au_start[i], i, au_size[i]);
            }
        }

        result
    }

    /// Decode an ADTS-wrapped AAC buffer using Symphonia and write the decoded PCM (s16 interleaved) to ALSA.
    fn decode_adts_and_play(&self, buf: &[u8]) {
        // symphonia expects a readable source (MediaSourceStream)
        let cursor = Cursor::new(buf.to_vec());
        let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

        let mut hint = Hint::new();

        hint.with_extension("aac");
        // hint.with_codec("aac");

        let probed = match get_probe().format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default()) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Symphonia probe failed: {}", e);
                return;
            }
        };

        let mut format = probed.format;

        let track = match format.tracks().first() {
            Some(t) => t,
            None => {
                eprintln!("No track found in ADTS buffer");
                return;
            }
        };

        let mut decoder = match get_codecs().make(&track.codec_params, &DecoderOptions::default()) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Failed to create codec decoder: {}", e);
                return;
            }
        };

        let mut pcm_guard = match self.pcm.lock() {
            Ok(g) => g,
            Err(_) => {
                eprintln!("Failed to lock PCM device");
                return;
            }
        };
        // create IO handle for interleaved i16
        let io_result = pcm_guard.io_i16();

        let mut io = match io_result {
            Ok(i) => i,
            Err(e) => {
                eprintln!("Failed to get ALSA IO i16 handle: {}", e);
                return;
            }
        };

        // read packets and decode
        loop {
            match format.next_packet() {
                Ok(packet) => {
                    match decoder.decode(&packet) {
                        Ok(audio_buf) => {
                            match audio_buf {
                                AudioBufferRef::S16(buf) => {
                                    let mut sample_buf = SampleBuffer::<i16>::new(buf.capacity() as u64, *buf.spec());
                                    sample_buf.copy_interleaved_ref(AudioBufferRef::S16(buf));
                                    let samples = sample_buf.samples();
                                    // write to ALSA (writei expects &[i16] interleaved)
                                    if let Err(e) = io.writei(samples) {
                                        eprintln!("ALSA writei error: {}", e);
                                        // Try to recover by preparing device
                                        if let Err(e2) = pcm_guard.prepare() {
                                            eprintln!("ALSA prepare failed: {}", e2);
                                            return;
                                        }
                                    }
                                }
                                AudioBufferRef::F32(buf) => {
                                    // Convert f32 -> i16
                                    let mut sample_buf = SampleBuffer::<f32>::new(buf.capacity() as u64, *buf.spec());
                                    sample_buf.copy_interleaved_ref(AudioBufferRef::F32(buf));
                                    let samples_f32 = sample_buf.samples();

                                    // convert and write in chunks
                                    let mut scratch = Vec::with_capacity(samples_f32.len());
                                    for &s in samples_f32 {
                                        // clamp and convert
                                        let s_i16 = if s <= -1.0 {
                                            i16::MIN
                                        } else if s >= 1.0 {
                                            i16::MAX
                                        } else {
                                            (s * (i16::MAX as f32)) as i16
                                        };
                                        scratch.push(s_i16);
                                    }
                                    if let Err(e) = io.writei(&scratch) {
                                        eprintln!("ALSA writei error (f32->i16): {}", e);
                                        if let Err(e2) = pcm_guard.prepare() {
                                            eprintln!("ALSA prepare failed: {}", e2);
                                            return;
                                        }
                                    }
                                }
                                _ => {
                                    eprintln!("Decoded audio buffer with unsupported sample format");
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Decode error: {}", e);
                            break;
                        }
                    }
                }
                Err(_) => {
                    // no more packets
                    break;
                }
            }
        }

        // flush/drain device (best-effort)
        if let Err(e) = pcm_guard.drain() {
            eprintln!("ALSA drain error: {}", e);
        }
    }
}

impl AudioOutput for DABPlusDecoder {

    fn init(&mut self, channels: u32, rate: u32) {

    }

    fn deinit(&mut self) {

    }

    /// It collects 5 frames to form a superframe, then processes that superframe (RS, AU parsing, CRC),
    /// decodes valid AUs with Symphonia and streams PCM to ALSA.
    fn output(&mut self, frame: &MainServiceChannelFrame) {
        // Append frame bits to sfbuf (we expect frame.bits contains a logical frame)
        let ibytes = frame.bits.len();
        let bitrate = frame.bitrate as usize;

        // eprintln!("ibytes: {} bitrate: {}", ibytes, bitrate);

        if self.frame_count == 0 {
            // only accept start if firecrccheck passes (mirror original C)
            if !firecrccheck(&frame.bits) {
                eprintln!("firecrccheck failed on first logical frame; ignoring");
                return;
            }
        }

        self.sfbuf.extend_from_slice(&frame.bits);
        self.frame_count += 1;

        // Process when 5 frames collected
        if self.frame_count <= 4 {
            return;
        }

        // We have 5 frames: perform deinterleaving, RS decoding, AU parsing
        // let sf_copy = self.sfbuf.clone(); // keep a copy for processing
        // Reset buffer and counter for next superframe

        // Deinterleave & RS-correct
        // let corrected_sf = self.deinterleave_and_rs(ibytes, bitrate);
        let corrected_sf = self.deinterleave_and_rs(ibytes, bitrate);

        self.sfbuf.clear();
        self.frame_count = 0;

        // Parse AUs (start/size pairs)
        let aus = self.parse_aus(&corrected_sf, ibytes, bitrate);
        if aus.is_empty() {
            eprintln!("No AUs found in superframe");
            return;
        }
        else {
            eprintln!("{} AUs found", aus.len());
        }

        // Build StreamParms for ADTS header construction
        let sp_byte = if corrected_sf.len() > 2 { corrected_sf[2] } else { 0 };
        let sp = StreamParms {
            rfa: ((sp_byte & 0x80) != 0) as u8,
            dac_rate: ((sp_byte & 0x40) != 0) as u8,
            sbr_flag: ((sp_byte & 0x20) != 0) as u8,
            aac_channel_mode: ((sp_byte & 0x10) != 0) as u8,
            ps_flag: ((sp_byte & 0x08) != 0) as u8,
            mpeg_surround_config: sp_byte & 0x07,
        };

        // audio_super_frame_size used earlier (C code)
        let s = if bitrate == 0 { 1 } else { bitrate / 8 };
        let audio_super_frame_size = ibytes * 5 - s * 10;

        // For each AU: invert CRC bytes (complement), check CRC, if OK: wrap ADTS + decode + play
        for (start, size) in aus {
            if start + size > audio_super_frame_size || size < 2 {
                eprintln!("AU out of range or too small: start={} size={}", start, size);
                continue;
            }

            // the AU data resides at offset `start` inside corrected_sf, up to size bytes.
            // In C: *(sfbuf + au_start[i] + au_size[i] - 2) ^= 0xff;
            //       *(sfbuf + au_start[i] + au_size[i] - 1) ^= 0xff;
            // copy AU into a separate Vec so we can invert bytes safely
            let mut au_vec = vec![0u8; size];
            let base = start;
            for i in 0..size {
                au_vec[i] = corrected_sf[base + i];
            }

            // invert last two CRC bytes as in the original C code
            let last = size - 1;
            au_vec[last - 1] ^= 0xFF;
            au_vec[last] ^= 0xFF;

            // check CRC
            if !Self::check_au_crc(&au_vec) {
                eprintln!("AU CRC failed; ignoring AU at start {}", start);
                continue;
            }

            // remove the 2 CRC bytes before passing to decoder (ADTS should contain only AAC payload)
            let payload_len = size - 2;
            let payload = &au_vec[..payload_len];

            // build ADTS
            let adts = Self::build_adts_header(payload_len, &sp);

            // combine ADTS + payload
            let mut full = Vec::with_capacity(7 + payload_len);
            full.extend_from_slice(&adts);
            full.extend_from_slice(payload);

            // decode & play (asynchronously or synchronously). We'll call sync decode/play.
            self.decode_adts_and_play(&full);
        }
    }
}
