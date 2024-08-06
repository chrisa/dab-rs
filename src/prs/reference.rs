use rustfft::num_complex::{c64, Complex64};
use std::str::FromStr;

// /* From ETSI EN 300 401 V1.3.3 Sect.14.3.2 Table 48 */
// const h: [[usize; 32]; 4] = [
//     [
//         0, 2, 0, 0, 0, 0, 1, 1, 2, 0, 0, 0, 2, 2, 1, 1, 0, 2, 0, 0, 0, 0, 1, 1, 2, 0, 0, 0, 2, 2,
//         1, 1,
//     ],
//     [
//         0, 3, 2, 3, 0, 1, 3, 0, 2, 1, 2, 3, 2, 3, 3, 0, 0, 3, 2, 3, 0, 1, 3, 0, 2, 1, 2, 3, 2, 3,
//         3, 0,
//     ],
//     [
//         0, 0, 0, 2, 0, 2, 1, 3, 2, 2, 0, 2, 2, 0, 1, 3, 0, 0, 0, 2, 0, 2, 1, 3, 2, 2, 0, 2, 2, 0,
//         1, 3,
//     ],
//     [
//         0, 1, 2, 1, 0, 3, 3, 2, 2, 3, 2, 1, 2, 1, 3, 2, 0, 1, 2, 1, 0, 3, 3, 2, 2, 3, 2, 1, 2, 1,
//         3, 2,
//     ],
// ];

// /* From ETSI EN 300 401 V1.3.3 Sect.14.3.2 Table 44 */
// const i: [usize; 48] = [
//     0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 3, 2, 1, 0, 3, 2, 1,
//     0, 3, 2, 1, 0, 3, 2, 1, 0, 3, 2, 1, 0, 3, 2, 1,
// ];

// const n: [usize; 48] = [
//     1, 2, 0, 1, 3, 2, 2, 3, 2, 1, 2, 3, 1, 2, 3, 3, 2, 2, 2, 1, 1, 3, 1, 2, 3, 1, 1, 1, 2, 2, 1, 0,
//     2, 2, 3, 3, 0, 2, 1, 3, 3, 3, 3, 0, 3, 0, 1, 1,
// ];

// const cospi2: [i32; 4] = [1, 0, -1, 0]; /* cos(0), cos(pi/2), cos(pi), cos(3*pi/2) */
// const sinpi2: [i32; 4] = [0, 1, 0, -1]; /* sin(0), sin(pi/2), sin(pi), sin(3*pi/2) */

// pub fn prs_reference() -> ([Complex64; 2048], [Complex64; 2048]) {
//     let mut prs_syms = [c64(0, 0); 2048];
//     let mut prs_conj = [c64(0, 0); 2048];

//     let mut j: usize = 0;
//     let mut kpl: i32 = -768;
//     let mut kp: i32;
//     let mut k: i32 = -768;
//     while k < 769 {
//         if k < 0 {
//             kp = if k % 32 == 0 { k } else { k - 32 - k % 32 };
//         } else {
//             kp = if k % 32 == 0 { k - 31 } else { k - k % 32 + 1 };
//         }

//         assert!(j < 48);
//         assert!(k - kp >= 0);
//         assert!(k - kp < 32);
//         let h_offset: usize = (k - kp) as usize;
//         let pi = h[i[j]][h_offset] + n[j];

//         let re = cospi2[pi % 4];
//         let im = sinpi2[pi % 4];

//         assert!(k > -(768 + 255));
//         let symbol_offset: usize = (k + 768 + 255) as usize;
//         prs_syms[symbol_offset] = c64(re, im);
//         prs_conj[symbol_offset] = c64(re, im).conj();

//         if kp != kpl && k != 0 {
//             kpl = kp;
//             j += 1;
//         }

//         k += 1;
//     }
//     prs_syms[768 + 255] = c64(0, 0); /* explicitly set value for k = 0 */
//     prs_conj[768 + 255] = c64(0, 0);

//     (prs_syms, prs_conj)
// }

const prs1_gplot: &str = include_str!("prs1.gplot");
const prs2_gplot: &str = include_str!("prs2.gplot");

pub fn prs_reference_1_2() -> ([Complex64; 2048], [Complex64; 2048]) {
    let prs1 = parse_gplot_file(prs1_gplot);
    let prs2 = parse_gplot_file(prs2_gplot);
    (prs1.try_into().unwrap(), prs2.try_into().unwrap())
}

fn parse_gplot_file(file: &str) -> Vec<Complex64>
{
    file.split("\n")
        .filter(|line| !line.is_empty())
        .map(parse_gplot_line)
        .collect::<Vec<Complex64>>()
}

fn parse_gplot_line(line: &str) -> Complex64 {
    let floats: Vec<&str> = line.split("  ").collect();
    let (re, im) = (f64::from_str(floats[0]), f64::from_str(floats[1]));
    c64(re.unwrap(), im.unwrap())
}
