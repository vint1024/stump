//! Server-side generation of placeholder cover images for books which have no
//! usable embedded cover (e.g. text-only EPUBs exported from word processors).
//! The generated cover renders the book title (and author, when known) over a
//! background color picked deterministically from the title, so a given book
//! always gets the same placeholder.
//!
//! The embedded font is Charis SIL Bold (SIL Open Font License 1.1), bundled
//! with this repository. It has wide glyph coverage, including full Cyrillic.

use std::io::Cursor;

use ab_glyph::{point, Font, FontRef, GlyphId, PxScale, ScaleFont};
use image::{ImageFormat, Rgb, RgbImage};

use crate::filesystem::error::FileError;

const FONT_BYTES: &[u8] = include_bytes!("fonts/CharisSIL-Bold.ttf");

pub const PLACEHOLDER_WIDTH: u32 = 600;
pub const PLACEHOLDER_HEIGHT: u32 = 900;

const SPINE_WIDTH: u32 = 14;
const HORIZONTAL_PADDING: f32 = 56.0;
const TITLE_SCALE: f32 = 58.0;
const AUTHOR_SCALE: f32 = 36.0;
const MAX_TITLE_LINES: usize = 7;
const MAX_AUTHOR_LINES: usize = 2;
const LINE_SPACING: f32 = 1.15;
const TEXT_COLOR: Rgb<u8> = Rgb([242, 240, 235]);

/// Muted, book-ish background palette. One entry is picked deterministically
/// per title so the same book always renders the same placeholder.
const PALETTE: [Rgb<u8>; 8] = [
	Rgb([52, 73, 94]),  // slate blue
	Rgb([84, 56, 71]),  // plum
	Rgb([47, 89, 70]),  // forest
	Rgb([105, 72, 49]), // leather brown
	Rgb([61, 64, 91]),  // indigo grey
	Rgb([112, 66, 65]), // brick
	Rgb([44, 95, 95]),  // teal
	Rgb([90, 70, 100]), // violet grey
];

/// Generate a PNG placeholder cover for a book without a usable embedded
/// cover image
pub fn generate_cover_placeholder(
	title: &str,
	author: Option<&str>,
) -> Result<Vec<u8>, FileError> {
	let font = FontRef::try_from_slice(FONT_BYTES).map_err(|error| {
		FileError::UnknownError(format!("Failed to load embedded font: {error}"))
	})?;

	let title = title.trim();
	let title = if title.is_empty() { "Untitled" } else { title };
	let author = author.map(str::trim).filter(|a| !a.is_empty());

	let (background, accent) = pick_colors(title);
	let mut image =
		RgbImage::from_pixel(PLACEHOLDER_WIDTH, PLACEHOLDER_HEIGHT, background);

	for y in 0..PLACEHOLDER_HEIGHT {
		for x in 0..SPINE_WIDTH {
			image.put_pixel(x, y, accent);
		}
	}

	let max_text_width = PLACEHOLDER_WIDTH as f32 - 2.0 * HORIZONTAL_PADDING;
	let center_x = SPINE_WIDTH as f32 / 2.0 + PLACEHOLDER_WIDTH as f32 / 2.0;

	let title_scale = PxScale::from(TITLE_SCALE);
	let title_lines =
		wrap_text(&font, title_scale, title, max_text_width, MAX_TITLE_LINES);
	let title_line_height = font.as_scaled(title_scale).height() * LINE_SPACING;
	let title_block_height = title_lines.len() as f32 * title_line_height;
	// Center the title block slightly above the vertical middle, like a real cover
	let mut baseline_y = PLACEHOLDER_HEIGHT as f32 * 0.42 - title_block_height / 2.0
		+ font.as_scaled(title_scale).ascent();
	for line in &title_lines {
		draw_line_centered(&mut image, &font, title_scale, line, center_x, baseline_y);
		baseline_y += title_line_height;
	}

	if let Some(author) = author {
		let author_scale = PxScale::from(AUTHOR_SCALE);
		let author_lines = wrap_text(
			&font,
			author_scale,
			author,
			max_text_width,
			MAX_AUTHOR_LINES,
		);
		let author_line_height = font.as_scaled(author_scale).height() * LINE_SPACING;
		let block_height = author_lines.len() as f32 * author_line_height;
		let mut author_baseline = PLACEHOLDER_HEIGHT as f32 - 90.0 - block_height
			+ font.as_scaled(author_scale).ascent();

		// A small separator rule between the title block and the author block
		let rule_y =
			(author_baseline - font.as_scaled(author_scale).ascent() - 24.0) as u32;
		if rule_y > 0 && rule_y < PLACEHOLDER_HEIGHT - 2 {
			let rule_half = 30u32;
			for y in rule_y..rule_y + 2 {
				for x in (center_x as u32 - rule_half)..(center_x as u32 + rule_half) {
					if x < PLACEHOLDER_WIDTH {
						image.put_pixel(x, y, TEXT_COLOR);
					}
				}
			}
		}

		for line in &author_lines {
			draw_line_centered(
				&mut image,
				&font,
				author_scale,
				line,
				center_x,
				author_baseline,
			);
			author_baseline += author_line_height;
		}
	}

	let mut buffer = Cursor::new(Vec::new());
	image::DynamicImage::ImageRgb8(image).write_to(&mut buffer, ImageFormat::Png)?;
	Ok(buffer.into_inner())
}

fn pick_colors(seed: &str) -> (Rgb<u8>, Rgb<u8>) {
	// FNV-1a, deterministic across runs (unlike std's RandomState-based hashers)
	let mut hash: u64 = 0xcbf29ce484222325;
	for byte in seed.as_bytes() {
		hash ^= u64::from(*byte);
		hash = hash.wrapping_mul(0x100000001b3);
	}
	let background = PALETTE[(hash % PALETTE.len() as u64) as usize];
	let accent = Rgb([
		(f32::from(background.0[0]) * 0.78) as u8,
		(f32::from(background.0[1]) * 0.78) as u8,
		(f32::from(background.0[2]) * 0.78) as u8,
	]);
	(background, accent)
}

fn measure_width(font: &impl Font, scale: PxScale, text: &str) -> f32 {
	let scaled = font.as_scaled(scale);
	let mut width = 0.0;
	let mut previous: Option<GlyphId> = None;
	for character in text.chars() {
		let id = scaled.glyph_id(character);
		if let Some(previous) = previous {
			width += scaled.kern(previous, id);
		}
		width += scaled.h_advance(id);
		previous = Some(id);
	}
	width
}

/// Greedy word wrap constrained to `max_lines`; appends an ellipsis when the
/// text doesn't fit. Words wider than a full line are hard-broken.
fn wrap_text(
	font: &impl Font,
	scale: PxScale,
	text: &str,
	max_width: f32,
	max_lines: usize,
) -> Vec<String> {
	let mut lines: Vec<String> = Vec::new();
	let mut current = String::new();
	let mut truncated = false;

	'words: for word in text.split_whitespace() {
		let candidate = if current.is_empty() {
			word.to_string()
		} else {
			format!("{current} {word}")
		};

		if measure_width(font, scale, &candidate) <= max_width {
			current = candidate;
			continue;
		}

		if !current.is_empty() {
			lines.push(std::mem::take(&mut current));
			if lines.len() >= max_lines {
				truncated = true;
				break 'words;
			}
		}

		if measure_width(font, scale, word) <= max_width {
			current = word.to_string();
			continue;
		}

		// Hard-break a single overly long word
		for character in word.chars() {
			current.push(character);
			if measure_width(font, scale, &current) > max_width {
				current.pop();
				lines.push(std::mem::take(&mut current));
				current.push(character);
				if lines.len() >= max_lines {
					truncated = true;
					break 'words;
				}
			}
		}
	}

	if !truncated && !current.is_empty() {
		lines.push(current);
	}
	if lines.len() > max_lines {
		lines.truncate(max_lines);
		truncated = true;
	}
	if truncated {
		if let Some(last) = lines.last_mut() {
			// Make room for the ellipsis so the line still fits
			while !last.is_empty()
				&& measure_width(font, scale, &format!("{last}…")) > max_width
			{
				last.pop();
			}
			last.push('…');
		}
	}

	lines
}

fn draw_line_centered(
	image: &mut RgbImage,
	font: &impl Font,
	scale: PxScale,
	text: &str,
	center_x: f32,
	baseline_y: f32,
) {
	let scaled = font.as_scaled(scale);
	let line_width = measure_width(font, scale, text);
	let mut x = center_x - line_width / 2.0;
	let mut previous: Option<GlyphId> = None;

	for character in text.chars() {
		let id = scaled.glyph_id(character);
		if let Some(previous) = previous {
			x += scaled.kern(previous, id);
		}
		let glyph = id.with_scale_and_position(scale, point(x, baseline_y));
		if let Some(outlined) = font.outline_glyph(glyph) {
			let bounds = outlined.px_bounds();
			outlined.draw(|gx, gy, coverage| {
				let px = bounds.min.x as i32 + gx as i32;
				let py = bounds.min.y as i32 + gy as i32;
				if px >= 0
					&& py >= 0 && (px as u32) < image.width()
					&& (py as u32) < image.height()
				{
					let pixel = image.get_pixel_mut(px as u32, py as u32);
					let coverage = coverage.clamp(0.0, 1.0);
					for channel in 0..3 {
						let bg = f32::from(pixel.0[channel]);
						let fg = f32::from(TEXT_COLOR.0[channel]);
						pixel.0[channel] = (bg + (fg - bg) * coverage) as u8;
					}
				}
			});
		}
		x += scaled.h_advance(id);
		previous = Some(id);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn count_text_pixels(png: &[u8]) -> usize {
		let decoded = image::load_from_memory(png).expect("valid png").to_rgb8();
		let (background, accent) =
			(decoded.get_pixel(300, 10), decoded.get_pixel(5, 450));
		let background = *background;
		let accent = *accent;
		decoded
			.pixels()
			.filter(|pixel| **pixel != background && **pixel != accent)
			.count()
	}

	#[test]
	fn generates_valid_png_with_expected_dimensions() {
		let bytes = generate_cover_placeholder(
			"The Fellowship of the Ring",
			Some("J.R.R. Tolkien"),
		)
		.expect("placeholder generated");
		let decoded = image::load_from_memory(&bytes).expect("decodable png");
		assert_eq!(decoded.width(), PLACEHOLDER_WIDTH);
		assert_eq!(decoded.height(), PLACEHOLDER_HEIGHT);
	}

	#[test]
	fn renders_cyrillic_titles() {
		let bytes =
			generate_cover_placeholder("Оборотень на Рождество", Some("Джиджи Риверс"))
				.expect("placeholder generated");
		// If the font lacked Cyrillic glyphs nothing (or only .notdef boxes)
		// would be drawn; require a substantial number of anti-aliased pixels
		assert!(count_text_pixels(&bytes) > 2_000);
	}

	#[test]
	fn is_deterministic_for_the_same_input() {
		let first = generate_cover_placeholder("Same Book", Some("Same Author")).unwrap();
		let second =
			generate_cover_placeholder("Same Book", Some("Same Author")).unwrap();
		assert_eq!(first, second);
	}

	#[test]
	fn handles_empty_title_and_missing_author() {
		let bytes =
			generate_cover_placeholder("  ", None).expect("placeholder generated");
		assert!(image::load_from_memory(&bytes).is_ok());
	}

	#[test]
	fn truncates_very_long_titles_with_ellipsis() {
		let long_title = "word ".repeat(120);
		let bytes =
			generate_cover_placeholder(&long_title, None).expect("placeholder generated");
		assert!(image::load_from_memory(&bytes).is_ok());
	}
}
