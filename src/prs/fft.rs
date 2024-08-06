use rustfft::FftPlanner;
use rustfft::num_complex::{c64, Complex64};

// static void vec_reverse_real(fftw_complex *vec, int pts)
// {
// 	/* The real part of the ifft has reverse order (apart from the
// 	   DC term) using fftw compared with the Intel SPL functions
// 	   used in the w!nd*ws software. */
        
// 	for (int i = 1; i < pts/2; i++) {
// 		double t = creal(*(vec+i));
// 		*(vec+i) = *(vec+i) - t + creal(*(vec+pts-i));
// 		*(vec+pts-i) = *(vec+pts-i) - creal(*(vec+pts-i)) + t;
// 	}
// }

fn reverse_real(data: &mut [Complex64; 2048])
{
    for i in 1..1024 {
        let t = data[i].re;
        data[i] = data[i] - t + data[2048 - i].re;
        data[2048 - i] = data[2048 - i] - data[2048 - i].re + t;
    }
}

// static void vec_reverse(fftw_complex *vec, int pts)
// {
// 	/* The real part of the fft has reverse order using fftw
// 	   compared with the Intel SPL functions used in the w!nd*ws
// 	   software */

//         for (int i = 1; i < pts/2; i++) {
//                 fftw_complex tc = *(vec+i);
//                 *(vec+i) = *(vec+pts-i);
//                 *(vec+pts-i) = tc;
//         }
// }

fn reverse(data: &mut [Complex64; 2048])
{
    for i in 1..1024 {
        // let tc = data[i];
        // data[i] = data[2048-i];
        // data[2048 - i] = tc;
        // std::mem::swap(&mut data[i], &mut data[2048 - i]);
        data.swap(i, 2048 - i);
    }
}

pub fn ifft(data: &[Complex64; 2048]) -> [Complex64; 2048] {
    let mut output = [c64(0, 0); 2048];
    output.clone_from_slice(data);
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_inverse(2048);
    fft.process(&mut output);
    // reverse_real(&mut output);
    output
}

pub fn fft(data: &[Complex64; 2048]) -> [Complex64; 2048] {
    let mut output = [c64(0, 0); 2048];
    output.clone_from_slice(data);
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(2048);
    fft.process(&mut output);
    reverse(&mut output);
    output
}