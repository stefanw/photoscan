use image::DynamicImage;
use image::GrayImage;
use imageproc::drawing::draw_line_segment_mut;
use imageproc::edges::canny;
use imageproc::hough::{detect_lines, LineDetectionOptions, PolarLine};
use wasm_bindgen::prelude::*;

pub struct ScanOptions {
    pub canny_low: f32,
    pub canny_high: f32,
    pub sigma_blur: f32,
    pub rdp_epsilon: f32,
    pub contour_threshold: u8,
    pub line_detection_options: LineDetectionOptions,
    pub debug: bool,
}

impl ScanOptions {
    pub fn default() -> ScanOptions {
        ScanOptions {
            canny_low: 10.0,
            canny_high: 80.0,
            sigma_blur: 2.0,
            rdp_epsilon: 1.0,
            contour_threshold: 200,
            line_detection_options: LineDetectionOptions {
                vote_threshold: 60,
                suppression_radius: 8,
            },
            debug: true,
        }
    }
}

#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub struct Point(pub u32, pub u32);

impl Point {
    fn distance(&self, other: &Point) -> f32 {
        euclidean_distance(self, other)
    }

    fn times_ratio(&self, ratio: f32) -> Point {
        Point(
            (self.0 as f32 * ratio) as u32,
            (self.1 as f32 * ratio) as u32,
        )
    }
}

#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub struct Quadrilateral {
    pub top_left: Point,
    pub top_right: Point,
    pub bottom_left: Point,
    pub bottom_right: Point,
}

impl Quadrilateral {
    pub fn new(
        top_left: Point,
        top_right: Point,
        bottom_left: Point,
        bottom_right: Point,
    ) -> Quadrilateral {
        Quadrilateral {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }
    pub fn times_ratio(&self, ratio: f32) -> Quadrilateral {
        Quadrilateral::new(
            self.top_left.times_ratio(ratio),
            self.top_right.times_ratio(ratio),
            self.bottom_left.times_ratio(ratio),
            self.bottom_right.times_ratio(ratio),
        )
    }
    pub fn translate(&self, point: &Point) -> Quadrilateral {
        Quadrilateral::new(
            Point(self.top_left.0 + point.0, self.top_left.1 + point.1),
            Point(self.top_right.0 + point.0, self.top_right.1 + point.1),
            Point(self.bottom_left.0 + point.0, self.bottom_left.1 + point.1),
            Point(self.bottom_right.0 + point.0, self.bottom_right.1 + point.1),
        )
    }
    pub fn area(&self) -> f32 {
        // area of quadrilateral by coordinates
        // A = 0.5 · (x1y2 - y1x2 + x2y3 - y2x3 + x3y4 - y3x4 + x4y1 - y4x1)
        0.5 * (self.top_left.0 * self.top_right.1 - self.top_left.1 * self.top_right.0
            + self.top_right.0 * self.bottom_right.1
            - self.top_right.1 * self.bottom_right.0
            + self.bottom_right.0 * self.bottom_left.1
            - self.bottom_right.1 * self.bottom_left.0
            + self.bottom_left.0 * self.top_left.1
            - self.bottom_left.1 * self.top_left.0) as f32
    }

    pub fn as_control_points(&self) -> [(f32, f32); 4] {
        [
            (self.top_left.0 as f32, self.top_left.1 as f32),
            (self.top_right.0 as f32, self.top_right.1 as f32),
            (self.bottom_right.0 as f32, self.bottom_right.1 as f32),
            (self.bottom_left.0 as f32, self.bottom_left.1 as f32),
        ]
    }

    pub fn draw_mut<C: imageproc::drawing::Canvas>(&self, image: &mut C, color: C::Pixel) {
        draw_line_segment_mut(
            image,
            (self.top_left.0 as f32, self.top_left.1 as f32),
            (self.top_right.0 as f32, self.top_right.1 as f32),
            color,
        );
        draw_line_segment_mut(
            image,
            (self.top_left.0 as f32, self.top_left.1 as f32),
            (self.bottom_left.0 as f32, self.bottom_left.1 as f32),
            color,
        );
        draw_line_segment_mut(
            image,
            (self.bottom_left.0 as f32, self.bottom_left.1 as f32),
            (self.bottom_right.0 as f32, self.bottom_right.1 as f32),
            color,
        );
        draw_line_segment_mut(
            image,
            (self.bottom_right.0 as f32, self.bottom_right.1 as f32),
            (self.top_right.0 as f32, self.top_right.1 as f32),
            color,
        );
    }
}

pub fn find_quadrilateral(
    input_image: &DynamicImage,
    options: &ScanOptions,
) -> Option<Quadrilateral> {
    // Prepare image
    log::debug!("Preparing image");
    let gray_image = input_image.blur(options.sigma_blur).to_luma8();

    // Create edge image using Canny algorithm
    log::debug!("Canny edge detection");
    let edges = canny(&gray_image, options.canny_low, options.canny_high);

    // Detect lines using Hough transform and extract prominent quadrilateral
    log::debug!("Hough transform");
    let result = find_hough_intersections(&edges, options);
    log::debug!("Result {:?}", result);
    result
}

pub fn find_hough_intersections(
    input_image: &GrayImage,
    options: &ScanOptions,
) -> Option<Quadrilateral> {
    let lines: Vec<PolarLine> = detect_lines(input_image, options.line_detection_options);

    // Cluster lines
    let angle_difference = 10;
    let r_difference = input_image.width().max(input_image.height()) as f32 * 0.05;

    let mut line_cluster: Vec<(PolarLine, Vec<&PolarLine>)> = vec![];
    lines.iter().for_each(|line| {
        let mut found = false;
        line_cluster.iter_mut().for_each(|item| {
            if line.angle_in_degrees.abs_diff(item.0.angle_in_degrees) < angle_difference
                && (line.r - item.0.r).abs() < r_difference
            {
                item.1.push(line);
                // Store updated representative PolarLine for the cluster
                let mean_angle: u32 =
                    item.1.iter().map(|l| l.angle_in_degrees).sum::<u32>() / item.1.len() as u32;
                let mean_r = item.1.iter().map(|l| l.r).sum::<f32>() / item.1.len() as f32;
                item.0 = PolarLine {
                    angle_in_degrees: mean_angle,
                    r: mean_r,
                };
                found = true
            }
        });
        if !found {
            line_cluster.push((*line, vec![line]));
        }
    });

    if line_cluster.len() < 4 {
        log::warn!("Found {} clusters, expected 4", line_cluster.len());
        return None;
    }

    // sort by cluster size
    line_cluster.sort_by(|a, b| a.1.len().cmp(&b.1.len()).reverse());

    // get first line from top 4 clusters (mean size is not working well in polar space)
    let top_4 = line_cluster
        .iter()
        .take(4)
        // .map(|item| *item.1[0])
        .map(|item| item.0)
        .collect::<Vec<_>>();

    // let lines_image = draw_polar_lines(&color_edges, &lines, green);
    // let mut lines_image = draw_polar_lines(&color_edges, &top_4, red);

    let mut intersections = vec![];
    top_4.iter().enumerate().for_each(|(idx1, line)| {
        top_4.iter().enumerate().for_each(|(idx2, other_line)| {
            if idx1 >= idx2 {
                return;
            }
            let intersection =
                polarline_intersection(line, other_line, input_image.width(), input_image.height());
            if let Some(intersection) = intersection {
                intersections.push(intersection);
            }
        });
    });

    log::debug!("Found intersections {:?}", intersections);

    if intersections.len() != 4 {
        return None;
    }

    // Assign named corners to intersections
    log::debug!("Found {:?} intersections", intersections);

    let mut sorted_by_x = intersections.clone();
    sorted_by_x.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    log::debug!("Sorted by x: {:?}", sorted_by_x);
    let mut left_most = sorted_by_x.iter().take(2).collect::<Vec<_>>();
    left_most.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    log::debug!("left most x: {:?}", left_most);

    let top_left = left_most[0];
    let bottom_left = left_most[1];

    let mut right_most = sorted_by_x.iter().rev().take(2).collect::<Vec<_>>();
    right_most.sort_by(|a, b| {
        euclidean_distance(a, top_left)
            .partial_cmp(&euclidean_distance(b, top_left))
            .unwrap()
    });

    log::debug!("right most x: {:?}", right_most);

    let top_right = right_most[0];
    let bottom_right = right_most[1];

    Some(Quadrilateral::new(
        Point(top_left.0, top_left.1),
        Point(top_right.0, top_right.1),
        Point(bottom_left.0, bottom_left.1),
        Point(bottom_right.0, bottom_right.1),
    ))
}

pub fn polarline_intersection(
    a: &PolarLine,
    b: &PolarLine,
    width: u32,
    height: u32,
) -> Option<Point> {
    let a_radians = (a.angle_in_degrees as f32).to_radians();
    let b_radians = (b.angle_in_degrees as f32).to_radians();
    let divisor = (a_radians - b_radians).sin();
    let x = (b.r * a_radians.sin() - a.r * b_radians.sin()) / divisor;
    let y = (a.r * b_radians.cos() - b.r * a_radians.cos()) / divisor;

    // Is the intersection point outside the image?
    if x < 0.0 || x > width as f32 || y < 0.0 || y > height as f32 {
        return None;
    }

    Some(Point(x as u32, y as u32))
}

pub fn euclidean_distance(a: &Point, b: &Point) -> f32 {
    let x = (a.0 as f32 - b.0 as f32).powi(2);
    let y = (a.1 as f32 - b.1 as f32).powi(2);
    (x + y).sqrt()
}

pub fn transform_quadrilateral(
    image: &image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    quadrilateral: &Quadrilateral,
    ratio: f32,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    // compute the width of the new image, which will be the
    // maximum distance between bottom-right and bottom-left
    // x-coordiates or the top-right and top-left x-coordinates

    let quadrilateral = quadrilateral.times_ratio(ratio);

    log::debug!("Transforming quadrilateral {:?}", quadrilateral);
    let width = quadrilateral
        .bottom_right
        .distance(&quadrilateral.bottom_left)
        .min(quadrilateral.top_right.distance(&quadrilateral.top_left));

    let height = quadrilateral
        .top_right
        .distance(&quadrilateral.bottom_right)
        .min(quadrilateral.top_left.distance(&quadrilateral.bottom_left));

    let to_dimensions = Quadrilateral::new(
        Point(0, 0),
        Point(width as u32, 0),
        Point(0, height as u32),
        Point(width as u32, height as u32),
    )
    .translate(&quadrilateral.top_left);

    log::debug!("Resulting quadrilateral {:?}", to_dimensions);

    let projection = imageproc::geometric_transformations::Projection::from_control_points(
        quadrilateral.as_control_points(),
        to_dimensions.as_control_points(),
    )
    .unwrap();
    log::debug!("Projection: {:?}", projection);

    let mut projected = imageproc::geometric_transformations::warp(
        image,
        &projection,
        imageproc::geometric_transformations::Interpolation::Nearest,
        image::Rgba([255, 0, 0, 255]),
    );
    let cropped = image::imageops::crop(
        &mut projected,
        quadrilateral.top_left.0,
        quadrilateral.top_left.1,
        width as u32,
        height as u32,
    );
    cropped.to_image()
}
