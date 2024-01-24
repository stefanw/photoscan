mod scan;
mod utils;

use image::DynamicImage;
use scan::{find_quadrilateral, Quadrilateral};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct TransformResult {
    pub width: u32,
    pub height: u32,
    buffer: Box<[u8]>,
}

#[wasm_bindgen]
impl TransformResult {
    #[wasm_bindgen(getter)]
    pub fn data(&self) -> js_sys::Uint8ClampedArray {
        js_sys::Uint8ClampedArray::from(&self.buffer[..])
    }
}

#[wasm_bindgen]
pub fn find_paper(width: u32, height: u32, buf: Box<[u8]>) -> Option<Quadrilateral> {
    wasm_logger::init(wasm_logger::Config::default());
    utils::set_panic_hook();
    let options = scan::ScanOptions::default();

    let buffer = image::RgbaImage::from_raw(width, height, buf.into()).unwrap();
    let image = DynamicImage::ImageRgba8(buffer);
    find_quadrilateral(&image, &options)
}

#[wasm_bindgen]
pub fn transform_paper(
    width: u32,
    height: u32,
    buf: Box<[u8]>,
    quadrilateral: Quadrilateral,
    ratio: f32,
) -> TransformResult {
    wasm_logger::init(wasm_logger::Config::default());
    utils::set_panic_hook();

    let buffer: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
        image::RgbaImage::from_raw(width, height, buf.into()).unwrap();
    // let quad = quadrilateral.unwrap_or_else(|| {
    //     Quadrilateral::new(
    //         scan::Point(0, 0),
    //         scan::Point(width, 0),
    //         scan::Point(width, height),
    //         scan::Point(0, height),
    //     )
    // });
    let result = scan::transform_quadrilateral(&buffer, &quadrilateral, ratio);
    let width = result.width();
    let height = result.height();
    TransformResult {
        width,
        height,
        buffer: result.into_raw().into_boxed_slice(),
    }
}
