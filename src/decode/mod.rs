use bitvec::prelude as bv;

mod viterbi;

pub use viterbi::new_viterbi;
pub use viterbi::Viterbi;

// For FIC symbol
// 384 u8 -> 3072 bits
pub type FicSymbol = bv::BitArr!(for 3072, in u8, bv::Msb0);

// 1152 u8 -> 9216 bits for merged symbols
pub type Fic = bv::BitArr!(for 9216, in u8, bv::Msb0);

// 288 u8 -> 2304 bits for split
pub type Fic2304 = bv::BitArr!(for 2304, in u8, bv::Msb0);

// 387 u8 -> 3096 bits for depuncture result
pub type Fic3096 = bv::BitArr!(for 3096, in u8, bv::Msb0);

// 96 u8 -> 768 bits for viterbi result
pub type Fic768 = bv::BitArr!(for 768, in u8, bv::Msb0);


pub fn byte_to_bit(bytes: [u8; 384]) -> FicSymbol {
    bytes.into()
}

pub fn bit_reverse(bits: &mut FicSymbol) {
    for chunk in bits.chunks_mut(16) {
        chunk.reverse();
    }
}
