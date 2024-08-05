#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod wavefinder;
use prs::PhaseReferenceSymbol;
use rustfft::{
    num_complex::{c64, Complex64},
    FftPlanner,
};
use wavefinder::{Buffer, Wavefinder};

mod prs;

fn ifft(data: [Complex64; 2048]) -> [Complex64; 2048] {
    let mut output = [c64(0, 0); 2048];
    output.clone_from_slice(&data);
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_inverse(2048);
    fft.process(&mut output);
    output
}

fn plot(data: [Complex64; 2048]) {
    for c in data {
        println!("{:10.3e}  {:10.3e}", c.re, c.im);
    }
    println!();
}

fn main() {
    let mut prs: PhaseReferenceSymbol = prs::new();
    let cb = move |buffer: Buffer| {
        prs.try_buffer(buffer);
        if prs.complete() {
            // dbg!(prs.vector());
            let i = ifft(prs.vector());
            plot(i);
            //dbg!(i);
            prs = prs::new();
        }
    };
    let w: Wavefinder = wavefinder::open(cb);
    w.init(225.648);
    w.read();
}
