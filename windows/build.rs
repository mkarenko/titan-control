//! Build script: rasterises `../assets/titan_control.svg` into Windows icons and embeds the executable's
//! resources (icon, version info, manifest).
//!
//! The icon is generated instead of committed so that the application keeps using the same artwork as the Linux
//! build. Recoloured copies match the tray icon styles: the logo of `TrayIconStyle::Theme` keeps the reference
//! colour, `Light` and `Dark` are white and black for dark and light taskbars.

use std::{env, fs, path::PathBuf};

/// Sizes stored in every icon file. Windows asks for 16 px at 100 % scaling and up to 48 px for large views.
const ICON_SIZES: [u32; 6] = [16, 20, 24, 32, 40, 48];
/// Supersampling factor per axis: 16 samples per pixel give smooth edges without anti-aliasing code.
const SAMPLES: u32 = 4;

fn main() {
    println!("cargo:rerun-if-changed=../assets/titan_control.svg");
    println!("cargo:rerun-if-changed=titan-control.manifest");

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let svg = fs::read_to_string("../assets/titan_control.svg").expect("read assets/titan_control.svg");
    let rings = flatten_path(&svg);

    // Reference colour of the Linux logo (`ColorScheme-Text`), then the monochrome tray styles.
    for (name, color) in [
        ("icon-theme", [0xBB, 0xCC, 0xDD]),
        ("icon-light", [0xFF, 0xFF, 0xFF]),
        ("icon-dark", [0x00, 0x00, 0x00]),
    ] {
        let path = out.join(format!("{name}.ico"));
        let alpha = render_alpha(&rings, *ICON_SIZES.iter().max().expect("sizes"));
        fs::write(&path, build_ico(&alpha, color)).expect("write icon");
        if name == "icon-theme" {
            embed_resources(&path);
        }
    }
}

fn embed_resources(icon: &PathBuf) {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon(icon.to_str().expect("icon path"));
    resource.set_manifest_file("titan-control.manifest");
    resource.set("FileDescription", "Titan Control");
    resource.set("ProductName", "Titan Control");
    resource.set("LegalCopyright", "MIT License");
    resource.set("OriginalFilename", "titan-control.exe");
    if let Err(error) = resource.compile() {
        println!("cargo:warning=could not embed the Windows resources: {error}");
    }
}

// ───────────────────────────── SVG path ─────────────────────────────

/// Extracts the `d` attribute of the first `<path>` and flattens every subpath into a polygon.
fn flatten_path(svg: &str) -> Vec<Vec<(f32, f32)>> {
    let start = svg.find("<path").expect("<path> element");
    let rest = &svg[start..];
    let d_start = rest.find(" d=\"").expect("path data");
    let rest = &rest[d_start + 4..];
    let d_end = rest.find('"').expect("closing quote");
    let data = &rest[..d_end];

    let mut rings = Vec::new();
    let mut points: Vec<(f32, f32)> = Vec::new();
    let mut cursor = (0.0f32, 0.0f32);
    let mut begin = (0.0f32, 0.0f32);
    let mut command = 'M';
    let mut tokens = tokens(data);
    let mut index = 0;

    while index < tokens.len() {
        let token = &tokens[index];
        if let Some(letter) = token.chars().next().filter(|c| c.is_ascii_alphabetic()) {
            if token.len() > 1 {
                // A number glued to its command letter (`c-2.577`).
                tokens[index] = token[1..].to_string();
            } else {
                index += 1;
            }
            command = letter;
            if command == 'z' || command == 'Z' {
                if points.len() > 2 {
                    points.push(begin);
                    rings.push(std::mem::take(&mut points));
                }
                cursor = begin;
                continue;
            }
            continue;
        }
        let number = |at: usize| -> f32 {
            tokens
                .get(at)
                .and_then(|token| token.parse::<f32>().ok())
                .unwrap_or(0.0)
        };
        match command {
            'M' | 'm' => {
                let (x, y) = (number(index), number(index + 1));
                index += 2;
                cursor = if command == 'm' { (cursor.0 + x, cursor.1 + y) } else { (x, y) };
                begin = cursor;
                if points.len() > 2 {
                    rings.push(std::mem::take(&mut points));
                }
                points = vec![cursor];
                // Extra coordinate pairs after a moveto are linetos (SVG 1.1 rule).
                command = if command == 'm' { 'l' } else { 'L' };
            }
            'l' | 'L' => {
                let (x, y) = (number(index), number(index + 1));
                index += 2;
                cursor = if command == 'l' { (cursor.0 + x, cursor.1 + y) } else { (x, y) };
                points.push(cursor);
            }
            'h' | 'H' => {
                let x = number(index);
                index += 1;
                cursor.0 = if command == 'h' { cursor.0 + x } else { x };
                points.push(cursor);
            }
            'v' | 'V' => {
                let y = number(index);
                index += 1;
                cursor.1 = if command == 'v' { cursor.1 + y } else { y };
                points.push(cursor);
            }
            'c' | 'C' => {
                let relative = command == 'c';
                let mut values = [0.0f32; 6];
                for (slot, value) in values.iter_mut().enumerate() {
                    *value = number(index + slot);
                }
                index += 6;
                let point = |x: f32, y: f32| -> (f32, f32) {
                    if relative { (cursor.0 + x, cursor.1 + y) } else { (x, y) }
                };
                let control1 = point(values[0], values[1]);
                let control2 = point(values[2], values[3]);
                let end = point(values[4], values[5]);
                flatten_cubic(cursor, control1, control2, end, &mut points);
                cursor = end;
            }
            _ => index += 1,
        }
    }
    if points.len() > 2 {
        rings.push(points);
    }
    rings
}

fn tokens(data: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in data.chars() {
        match character {
            ' ' | ',' | '\n' | '\r' | '\t' => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            '-' | '+' => {
                // A sign starts a new number unless it follows an exponent marker.
                if current.is_empty() || current.ends_with('e') || current.ends_with('E') {
                    current.push(character);
                } else {
                    tokens.push(std::mem::take(&mut current));
                    current.push(character);
                }
            }
            _ => current.push(character),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn flatten_cubic(from: (f32, f32), c1: (f32, f32), c2: (f32, f32), to: (f32, f32), out: &mut Vec<(f32, f32)>) {
    const STEPS: usize = 24;
    for step in 1..=STEPS {
        let t = step as f32 / STEPS as f32;
        let mt = 1.0 - t;
        let a = mt * mt * mt;
        let b = 3.0 * mt * mt * t;
        let c = 3.0 * mt * t * t;
        let d = t * t * t;
        out.push((
            a * from.0 + b * c1.0 + c * c2.0 + d * to.0,
            a * from.1 + b * c1.1 + c * c2.1 + d * to.1,
        ));
    }
}

// ───────────────────────────── rasteriser ─────────────────────────────

/// Renders the rings (256 × 256 user units) into a `size` × `size` coverage bitmap, even-odd rule.
fn render_alpha(rings: &[Vec<(f32, f32)>], size: u32) -> AlphaBitmap {
    let high = size * SAMPLES;
    let scale = high as f32 / 256.0;
    let mut coverage = vec![0u16; (high * high) as usize];

    // Pixel centres of the supersampled grid.
    for row in 0..high {
        let y = (row as f32 + 0.5) / scale;
        let mut crossings: Vec<f32> = Vec::with_capacity(64);
        for ring in rings {
            for window in ring.windows(2) {
                let (x0, y0) = window[0];
                let (x1, y1) = window[1];
                if (y0 <= y && y < y1) || (y1 <= y && y < y0) {
                    crossings.push(x0 + (y - y0) * (x1 - x0) / (y1 - y0));
                }
            }
        }
        if crossings.len() < 2 {
            continue;
        }
        crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        for pair in crossings.chunks_exact(2) {
            let from = ((pair[0] * scale).round() as i64).clamp(0, high as i64) as u32;
            let to = ((pair[1] * scale).round() as i64).clamp(0, high as i64) as u32;
            let base = (row * high) as usize;
            for column in from..to {
                coverage[base + column as usize] += 1;
            }
        }
    }

    let samples = (SAMPLES * SAMPLES) as u16;
    let mut pixels = vec![0u8; (size * size) as usize];
    for y in 0..size {
        for x in 0..size {
            let mut sum = 0u32;
            for dy in 0..SAMPLES {
                for dx in 0..SAMPLES {
                    let row = y * SAMPLES + dy;
                    let column = x * SAMPLES + dx;
                    let value = coverage[(row * high + column) as usize].min(samples);
                    sum += value as u32 * 255 / samples as u32;
                }
            }
            pixels[(y * size + x) as usize] = (sum / (SAMPLES * SAMPLES)) as u8;
        }
    }
    AlphaBitmap { size, pixels }
}

struct AlphaBitmap {
    size: u32,
    pixels: Vec<u8>,
}

impl AlphaBitmap {
    /// Box-filter downscale (the source is a multiple of the destination because `ICON_SIZES` divides 48).
    fn resized(&self, size: u32) -> Vec<u8> {
        if size == self.size {
            return self.pixels.clone();
        }
        let factor = self.size / size;
        let mut out = vec![0u8; (size * size) as usize];
        for y in 0..size {
            for x in 0..size {
                let mut sum = 0u32;
                for dy in 0..factor {
                    for dx in 0..factor {
                        let sx = x * factor + dx;
                        let sy = y * factor + dy;
                        sum += self.pixels[(sy * self.size + sx) as usize] as u32;
                    }
                }
                out[(y * size + x) as usize] = (sum / (factor * factor)) as u8;
            }
        }
        out
    }
}

// ───────────────────────────── ICO writer ─────────────────────────────

fn build_ico(alpha: &AlphaBitmap, color: [u8; 3]) -> Vec<u8> {
    let mut images: Vec<(u32, Vec<u8>)> = Vec::new();
    for size in ICON_SIZES {
        images.push((size, dib(alpha.resized(size), size, color)));
    }

    let mut out = Vec::with_capacity(64 * 1024);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, data) in &images {
        out.push(if *size >= 256 { 0 } else { *size as u8 });
        out.push(if *size >= 256 { 0 } else { *size as u8 });
        out.push(0);
        out.push(0);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += data.len() as u32;
    }
    for (_, data) in images {
        out.extend_from_slice(&data);
    }
    out
}

/// A 32-bit bottom-up DIB with an AND mask, the layout RT_ICON resources use.
fn dib(pixels: Vec<u8>, size: u32, color: [u8; 3]) -> Vec<u8> {
    let mask_row = size.div_ceil(32) * 4;
    let mask_size = mask_row * size;
    let image_size = size * size * 4;
    let mut out = Vec::with_capacity(40 + (image_size + mask_size) as usize);

    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(size as i32).to_le_bytes());
    out.extend_from_slice(&((size * 2) as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(image_size + mask_size).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());

    // XOR bitmap: BGRA, bottom-up, premultiplied by nothing (Windows expects straight alpha).
    for row in (0..size).rev() {
        for column in 0..size {
            let alpha = pixels[(row * size + column) as usize];
            out.push(color[2]);
            out.push(color[1]);
            out.push(color[0]);
            out.push(alpha);
        }
    }
    // AND mask: 1 = transparent, for viewers that ignore the alpha channel.
    for row in (0..size).rev() {
        let mut bits = vec![0u8; mask_row as usize];
        for column in 0..size {
            if pixels[(row * size + column) as usize] < 128 {
                bits[(column / 8) as usize] |= 0x80 >> (column % 8);
            }
        }
        out.extend_from_slice(&bits);
    }
    out
}
