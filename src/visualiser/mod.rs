use std::{
    borrow::{Borrow, BorrowMut},
    ops::Range,
};

mod pixel_buf;
mod window;

use plotters::{
    prelude::Circle,
    style::{Color, BLACK, CYAN},
};
use rustfft::num_complex::Complex64;

use pixel_buf::PixelBuf;
use window::{get_drawing_area, setup_window, Cartesian2d, ChartState, RangedCoordf64, Window};

pub struct Visualiser {
    window: Window,
    cs: ChartState<Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    pixel_buf: PixelBuf,
    height: usize,
    width: usize,
}

pub fn create_visualiser(
    name: &str,
    height: usize,
    width: usize,
    x_range: Range<f64>,
    y_range: Range<f64>,
) -> Visualiser {
    let (mut window, cs, pixel_buf) =
        setup_window(name, height, width, x_range, y_range, "real", "imag");
    window.set_target_fps(144);

    Visualiser {
        window,
        cs,
        pixel_buf,
        height,
        width,
    }
}

impl Visualiser {
    pub fn update(&mut self, data: [Complex64; 2048]) {
        let drawing_area = get_drawing_area(self.pixel_buf.borrow_mut(), self.width, self.height);
        let mut chart = self.cs.clone().restore(&drawing_area);
        chart.plotting_area().fill(&BLACK).borrow();

        // draw
        let data_iter = data.map(|c| (c.re, c.im));

        chart
            .draw_series(data_iter.map(|(x, y)| Circle::new((x, y), 1, CYAN.filled())))
            .unwrap();
        drop(drawing_area);
        drop(chart);

        self.window
            .update_with_buffer(self.pixel_buf.borrow(), self.width, self.height)
            .unwrap();
    }
}
