use crate::visualiser::pixel_buf::PixelBuf;
use minifb::WindowOptions;
use plotters::chart::ChartBuilder;
use plotters::coord::Shift;
use plotters::drawing::{DrawingArea, IntoDrawingArea};
use plotters::style::{IntoFont, WHITE};
use plotters_bitmap::bitmap_pixel::BGRXPixel;
use plotters_bitmap::BitMapBackend;
use std::borrow::BorrowMut;
use std::ops::Range;

pub use minifb::Window;
pub use plotters::chart::ChartState;
pub use plotters::coord::cartesian::Cartesian2d;
pub use plotters::coord::types::RangedCoordf64;

pub fn setup_window(
    name: &str,
    height: usize,
    width: usize,
    x_range: Range<f64>,
    y_range: Range<f64>,
    x_desc: &str,
    y_desc: &str,
) -> (
    Window,
    ChartState<Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    PixelBuf,
) {
    let window = Window::new(&String::from(name), width, height, WindowOptions::default()).unwrap();

    // Buffer where we draw the Chart as bitmap into: we update the "minifb" window from it too
    let mut pixel_buf = PixelBuf(vec![0_u32; width * height]);

    let drawing_area = get_drawing_area(pixel_buf.borrow_mut(), width, height);

    let chart = draw_chart(drawing_area, x_range, y_range, x_desc, y_desc);

    (window, chart, pixel_buf)
}

pub fn get_drawing_area(
    pixel_buf: &mut [u8],
    width: usize,
    height: usize,
) -> DrawingArea<BitMapBackend<BGRXPixel>, Shift> {
    // BGRXPixel format required by "minifb" (alpha, red, green, blue)
    let drawing_area = BitMapBackend::<BGRXPixel>::with_buffer_and_format(
        pixel_buf.borrow_mut(),
        (width as u32, height as u32),
    )
    .unwrap()
    .into_drawing_area();

    drawing_area
}

fn draw_chart<'a>(
    drawing_area: DrawingArea<BitMapBackend<BGRXPixel>, Shift>,
    x_range: Range<f64>,
    y_range: Range<f64>,
    x_desc: &'a str,
    y_desc: &'a str,
) -> ChartState<Cartesian2d<RangedCoordf64, RangedCoordf64>> {
    let mut chart = ChartBuilder::on(&drawing_area)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(x_range, y_range)
        .unwrap();

    chart
        .configure_mesh()
        .label_style(("sans-serif", 15).into_font().color(&WHITE))
        .disable_x_mesh()
        .disable_y_mesh()
        .x_desc(x_desc)
        .y_desc(y_desc)
        .x_labels(10)
        .y_labels(10)
        .draw()
        .unwrap();

    chart.into_chart_state()
}
