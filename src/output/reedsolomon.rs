// at top of your file
use reed_solomon_rs::fec::fec::{FEC, Share};
use std::error::Error;

/// Call this once at initialization:
pub fn make_fec() -> Result<FEC, Box<dyn Error>> {
    // K = 110 (required), N = 120 (total)
    // This matches KA9Q parameters: N=120, K=110, nroots=10
    let required = 110usize;
    let total = 120usize;
    let fec = FEC::new(required, total)?;
    Ok(fec)
}

/// Convert a codeword cbuf (len == 120) into shares vector the crate expects
fn cbuf_to_shares(cbuf: &[u8]) -> Vec<Share> {
    // each share will hold a 1-byte payload
    let mut shares = Vec::with_capacity(cbuf.len());
    for (i, &b) in cbuf.iter().enumerate() {
        shares.push(Share {
            number: i,       // share index (0..119)
            data: vec![b],   // one byte per share
        });
    }
    shares
}

/// Map the reed_solomon_rs decode result back into the sfbuf like KA9Q
/// - fec: prepared FEC instance
/// - cbuf: mutable buffer length 120 (filled from deinterleaving)
/// - sfbuf_write_back: closure to write j-th data byte into sfbuf position s*j + i
pub fn try_rs_decode_and_write<F>(
    fec: &FEC,
    cbuf: &mut [u8; 120],
    mut write_back: F,
) where
    F: FnMut(usize, u8), // (j, data_byte) -> writes sfbuf[s*j + i] = data_byte
{
    // Build shares
    let shares = cbuf_to_shares(&cbuf[..]);

    // No erasures known, pass empty vector for erasure indices
    let missing_shares: Vec<u8> = Vec::new();

    match fec.decode(missing_shares, shares) {
        Ok(recovered) => {
            // `recovered` contains required * unit_size bytes. unit_size == 1 here.
            // The first K (=required) bytes are the original message data.
            // Copy first 110 bytes back (replicates writing back cbuf[0..109])
            for j in 0..110usize {
                let byte = recovered[j];
                write_back(j, byte);
            }
            // Optionally, return number of corrected symbols if fec exposes it.
        }
        Err(e) => {
            // decoding failed: same behavior as KA9Q/C code — leave original bytes unchanged
            eprintln!("RS decode failed: {:?}", e);
        }
    }
}
