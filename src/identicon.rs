use image::{ImageBuffer, Rgb, RgbImage};
use sha2::{Digest, Sha256};

const GRID: u32 = 5;
const CELL: u32 = 70;
const SIZE: u32 = GRID * CELL;
const BG: Rgb<u8> = Rgb([240, 240, 240]);

/// Generate a simple symmetric identicon PNG from a seed string.
/// Returns raw PNG bytes, or an empty vec on error.
pub fn generate_identicon(seed: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let hash = hasher.finalize();

    let fg = hash_to_color(hash[0], hash[1]);
    let mut img: RgbImage = ImageBuffer::from_pixel(SIZE, SIZE, BG);

    // Rows 0..5, cols 0..3 (col 3/4 are mirrors of 1/0)
    for row in 0..GRID {
        for col in 0..3u32 {
            let idx = (row * 3 + col + 1) as usize % 32;
            if hash[idx] & 1 == 1 {
                fill_cell(&mut img, col, row, fg);
                if col < 2 {
                    fill_cell(&mut img, GRID - 1 - col, row, fg);
                }
            }
        }
    }

    let mut buf = Vec::new();
    let cursor = std::io::Cursor::new(&mut buf);
    let encoder = image::codecs::png::PngEncoder::new(cursor);
    if let Err(_) = image::ImageEncoder::write_image(encoder, img.as_raw(), SIZE, SIZE, image::ExtendedColorType::Rgb8)
    {
        return Vec::new();
    }
    buf
}

fn fill_cell(img: &mut RgbImage, col: u32, row: u32, color: Rgb<u8>) {
    let x0 = col * CELL;
    let y0 = row * CELL;
    for y in y0..y0 + CELL {
        for x in x0..x0 + CELL {
            img.put_pixel(x, y, color);
        }
    }
}

/// Convert two hash bytes into an RGB foreground color using HSL.
fn hash_to_color(h_byte: u8, s_byte: u8) -> Rgb<u8> {
    let hue = (h_byte as f32 / 255.0) * 360.0;
    let sat = 0.45 + (s_byte as f32 / 255.0) * 0.3; // range 0.45–0.75
    hsl_to_rgb(hue, sat, 0.50)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> Rgb<u8> {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0f32)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    Rgb([
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    ])
}
