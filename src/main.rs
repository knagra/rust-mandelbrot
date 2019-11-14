extern crate num;
extern crate image;
extern crate crossbeam;

use image::ColorType;
use image::png::PNGEncoder;
use num::Complex;
use std::fs::File;
use std::io::Write;
use std::str::FromStr;

/// Determine if the Mandelbrot sequence escapes the finite attraction basin
/// using limit as the iteration limit.
///
/// This function will compute the sequence f(c), f(f(c)), f(f(f(c))), etc.
/// up to limit times and see if it escapes the attraction basin at 0.
/// If it does, this function will return the number of iterations it took
/// for the sequence to escape as Some(i).
/// If it does not, this function will return None.
fn escape_time(c: Complex<f64>, limit: u32) -> Option<u32> {
    let mut z = Complex { re: 0.0, im: 0.0 };
    for i in 0..limit {
        z = z * z + c;
        if z.norm_sqr() > 4.0 {
            return Some(i);
        }
    }

    None
}

/// Parse a pair of coordinates s from a string given a separator.
///
/// E.g., parse_pair("3x5", 'x') -> Some((3, 5)).
/// E.g., parse_pair("3.0,5.0", ',') -> Some((3.0, 5.0)).
///
/// If s is properly formatted, return the coordinates as Some(x, y).
/// If s is no properly formatted, return None.
fn parse_pair<T: FromStr>(s: &str, separator: char) -> Option<(T, T)> {
    match s.find(separator) {
        None => None,
        Some(index) => {
            match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) {
                (Ok(l), Ok(r)) => Some((l, r)),
                _ => None
            }
        }
    }
}

#[test]
fn test_parse_pair() {
    assert_eq!(parse_pair::<i32>("", ','), None);
    assert_eq!(parse_pair::<i32>("10,", ','), None);
    assert_eq!(parse_pair::<i32>(",10", ','), None);
    assert_eq!(parse_pair::<f64>("10.0,20.0", ','), Some((10, 20)));
    assert_eq!(parse_pair::<i32>("10,20xy", ','), None);
    assert_eq!(parse_pair::<f64>("0.5x", 'x'), None);
    assert_eq!(parse_pair::<f64>("0.5x1.5", 'x'), Some((0.5, 1.5)));
}

/// Parse a pair of floats separated by a comma as a complex number.
fn parse_complex(s: &str) -> Option<Complex<f64>> {
    match parse_pair(s, ',') {
        Some((re, im)) => Some(Complex { re, im }),
        None => None
    }
}

#[test]
fn test_parse_complex() {
    assert_eq!(parse_complex("1.25,-0.0625"), Some(Complex { re: 1.25, im: -0.0625 }));
    assert_eq!(parse_complex(",-0.0625"), None);
}

/// Given the row and column of a pixel in the output image, return the corresponding
/// point on the complex plane.
///
/// `pixel_width` is the width of the image in pixels.
/// `pixel_height` is the height of the image in pixels.
/// `pixel` is a (column, row) pair indicating a particular pixel in that image.
/// The `upper_left` and `lower_right` parameters are points on the complex plane
/// designating the area our image covers.
fn pixel_to_point(
    pixel_width: usize, pixel_height: usize, target_pixel: (usize, usize),
    upper_left: Complex<f64>, lower_right: Complex<f64>
) -> Complex<f64> {
    let (width, height) = (
        lower_right.re - upper_left.re,
        upper_left.im - lower_right.im
    );
    Complex {
        re: upper_left.re + target_pixel.0 as f64 * width / pixel_width as f64,
        im: upper_left.im - target_pixel.1 as f64 * height / pixel_height as f64
    }
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(
        pixel_to_point(
            100, 100, (25, 75), Complex { re: -1.0, im: 1.0 },
            Complex { re: 1.0, im: -1.0}
        ),
        Complex { re: -0.5, im: -0.5 }
    );
}

/// Render a rectangle of the Mandelbrot set into a buffer of pixels.
///
/// The `pixel_width` and `pixel_height` arguments give the width and height in pixels
/// of the `pixels` buffer, which holds one grayscale pixel per byte. The `upper_left`
/// and `lower_right` arguments specify points on the complex plane corresponding to the
/// upper-left and lower-right corners of the pixel buffer.
fn render(
    pixels: &mut [u8], pixel_width: usize, pixel_height: usize, upper_left: Complex<f64>,
    lower_right: Complex<f64>
) {
    assert!(pixels.len() == pixel_width * pixel_height * 3);

    for row in 0..pixel_height {
        for column in 0..pixel_width {
            let point = pixel_to_point(
                pixel_width, pixel_height, (column, row), upper_left, lower_right
            );
            match escape_time(point, 255) {
                None => {
                    pixels[row * pixel_width * 3 + column * 3] = 0;
                    pixels[row * pixel_width * 3 + column * 3 + 1] = 0;
                    pixels[row * pixel_width * 3 + column * 3 + 2] = 0;
                },
                Some(count) => {
                    pixels[row * pixel_width * 3 + column * 3] = 255 - count as u8;
                    pixels[row * pixel_width * 3 + column * 3 + 1] =
                        (column * 255 / pixel_width) as u8;
                    pixels[row * pixel_width * 3 + column * 3 + 2] =
                        (255 - column * 255 / pixel_width) as u8;
                }
            };
            /*
            pixels[row * pixel_width + column * 3] =
                match escape_time(point, 255) {
                    None => 0,
                    Some(count) => 255 - count as u8
                };
            pixels[row * pixel_width + column * 3 + 1] = 0;
            pixels[row * pixel_width + column * 3 + 2] = 0;
            */
        }
    }
}

/// Write the buffer `pixels`, whose dimensions are given by `pixel_width` & 
/// `pixel_height`, to the file name `filename`.
fn write_image(filename: &str, pixels: &[u8], pixel_width: usize, pixel_height: usize)
        -> Result<(), std::io::Error> {
    let output = File::create(filename)?;

    let encoder = PNGEncoder::new(output);
    encoder.encode(
        &pixels,
        pixel_width as u32,
        pixel_height as u32,
        ColorType::RGB(8)
    )?;
    Ok(())
}

#[allow(dead_code)]
fn synchronous(
    pixels: &mut [u8], pixel_width: usize, pixel_height: usize, upper_left: Complex<f64>,
    lower_right: Complex<f64>
) {
    render(pixels, pixel_width, pixel_height, upper_left, lower_right);
}

fn concurrent(
    pixels: &mut [u8], pixel_width: usize, pixel_height: usize, upper_left: Complex<f64>,
    lower_right: Complex<f64>
) {
    let threads = 12;
    let rows_per_band = pixel_height / threads + 1;

    {
        let bands: Vec<&mut [u8]> = pixels.chunks_mut(rows_per_band * pixel_width * 3)
                .collect();
        crossbeam::scope(|spawner| {
            for (i, band) in bands.into_iter().enumerate() {
                let top = rows_per_band * i;
                let band_height = band.len() / (pixel_width * 3);
                let band_upper_left = pixel_to_point(
                    pixel_width, pixel_height, (0, top), upper_left, lower_right
                );
                let band_lower_right = pixel_to_point(
                    pixel_width, pixel_height, (pixel_width, top + band_height),
                    upper_left, lower_right
                );

                spawner.spawn(move |_| {
                    render(
                        band, pixel_width, band_height, band_upper_left, band_lower_right
                    );
                });
            }
        });
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 5 {
        writeln!(
            std::io::stderr(),
            "Usage: mandelbrot FILE PIXELS UPPERLEFT LOWERRIGHT"
        ).unwrap();
        writeln!(
            std::io::stderr(),
            "Example: {} mandel.png 1000x750 -1.20,0.35 -1,0.20",
            args[0]
        ).unwrap();
        std::process::exit(1);
    }
    let (pixel_width, pixel_height) = parse_pair(&args[2], 'x').expect(
        "error parsing image dimensions"
    );
    let upper_left = parse_complex(&args[3]).expect(
        "error parsing upper left corner point"
    );
    let lower_right = parse_complex(&args[4]).expect(
        "error parsing lower right corner point"
    );

    let mut pixels = vec![0; pixel_width * pixel_height * 3];
    // synchronous(&mut pixels, pixel_width, pixel_height, upper_left, lower_right);
    concurrent(&mut pixels, pixel_width, pixel_height, upper_left, lower_right);

    write_image(&args[1], &pixels, pixel_width, pixel_height).expect(
        "error writing PNG file"
    );
}
