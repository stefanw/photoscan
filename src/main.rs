use crate::scan::{find_hough_intersections, find_quadrilateral, ScanOptions};
use image::{open, Rgb};
use imageproc::edges::canny;
use imageproc::hough::draw_polar_lines;
use imageproc::map::map_colors;
use std::env;
use std::fs;
use std::path::Path;

mod scan;

fn main() {
    env_logger::init();

    let options = ScanOptions::default();

    if env::args().len() != 3 {
        panic!("Please enter an input file and a target directory")
    }

    let input_path = env::args().nth(1).unwrap();
    let output_dir = env::args().nth(2).unwrap();

    let input_path = Path::new(&input_path);
    let output_dir = Path::new(&output_dir);

    if !output_dir.is_dir() {
        fs::create_dir(output_dir).expect("Failed to create output directory")
    }

    if !input_path.is_file() {
        panic!("Input file does not exist");
    }

    // Load image and convert to grayscale
    let original_image =
        open(input_path).expect(&format!("Could not load image at {:?}", input_path));

    let input_image = original_image.blur(options.sigma_blur).to_luma8();

    // Save grayscale image in output directory
    let gray_path = output_dir.join("grey.png");
    input_image.save(&gray_path).unwrap();

    // Detect edges using Canny algorithm
    let edges = canny(&input_image, options.canny_low, options.canny_high);
    let canny_path = output_dir.join("canny.png");
    edges.save(&canny_path).unwrap();

    let white = Rgb::<u8>([255, 255, 255]);
    let green = Rgb::<u8>([0, 255, 0]);
    let red = Rgb::<u8>([255, 0, 0]);
    let black = Rgb::<u8>([0, 0, 0]);

    // let green = Luma([255u8]);

    // Convert edge image to colour
    let mut color_edges = map_colors(&edges, |p| if p[0] > 0 { white } else { black });

    // Detect lines using Hough transform
    // let intersections = find_quadrilateral(&open(input_path).unwrap(), &options);

    let lines: Vec<imageproc::hough::PolarLine> =
        imageproc::hough::detect_lines(&edges, options.line_detection_options);
    let polar_lines = draw_polar_lines(&color_edges, &lines, red);
    log::info!("Found polar lines {:?}", lines);
    let lines_path = output_dir.join("polar_lines.png");
    polar_lines.save(&lines_path).unwrap();

    let intersections = find_hough_intersections(&edges, &options);

    if let Some(points) = intersections {
        log::info!("Found four points {:?}", points);
        points.draw_mut(&mut color_edges, green);

        let original_color = original_image.to_rgba8();
        let result = scan::transform_quadrilateral(&original_color, &points, 1.0);
        result.save(output_dir.join("result.png")).unwrap();
    }

    // Save lines image in output directory
    let lines_path = output_dir.join("lines.png");
    color_edges.save(&lines_path).unwrap();
}
