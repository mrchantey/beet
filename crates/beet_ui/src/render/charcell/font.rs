//! Glyph scaling for the charcell renderer, driven by the resolved `font-size`.
//!
//! Three rendering modes (see [`FontScale`]):
//! - `<= 1em` **Normal**: one cell per ASCII glyph (the default path).
//! - `> 1em`  **Wide**: each glyph maps to its fullwidth twin (`A` -> `Ａ`), so
//!   text reads twice as wide. This is a pure character remap, so it reuses the
//!   normal measure/paint path — fullwidth glyphs already measure as 2 columns.
//! - `> 2em`  **Block**: the multi-row box-drawing font in `font.txt`, 3 rows
//!   tall, with uppercase letters drawn in the double-pipe variant. This has a
//!   dedicated path: [`measure_block_text`] and [`paint_block_text`], dispatched
//!   from the measure and paint phases.
use super::AsBuffer;
use super::Clip;
use super::align_offset;
use super::display_width;
use crate::style::Length;
use crate::style::TextAlign;
use crate::style::VisualStyle;
use beet_core::prelude::*;
use bevy::math::IRect;
use bevy::math::IVec2;
use bevy::math::UVec2;
use bevy::math::Vec2;
use std::sync::LazyLock;

/// How the charcell renderer scales a run of text, selected from its resolved
/// [`font-size`](VisualStyle::font_size).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontScale {
	/// `<= 1em`: one cell per glyph, the unscaled default.
	Normal,
	/// `> 1em`: fullwidth glyphs, two cells wide (see [`to_fullwidth`]).
	Wide,
	/// `> 2em`: the multi-row box-drawing block font.
	Block,
}

impl FontScale {
	/// `font-size` (in em/rem) above which glyphs render fullwidth.
	pub const WIDE_EM: f32 = 1.0;
	/// `font-size` (in em/rem) above which glyphs render as the block font.
	pub const BLOCK_EM: f32 = 2.0;

	/// Resolve the scale from a resolved font-size. `1rem == 1 cell`, so the em
	/// thresholds compare directly against the rem value.
	pub fn from_font_size(font_size: Length, viewport: Vec2) -> Self {
		let em = font_size.into_rem(viewport);
		if em > Self::BLOCK_EM {
			Self::Block
		} else if em > Self::WIDE_EM {
			Self::Wide
		} else {
			Self::Normal
		}
	}

	/// Resolve the scale from a [`VisualStyle`]. `font-size` is
	/// resolution-independent (rem/px in practice), so no viewport is needed; a
	/// viewport-relative font-size is unsupported and resolves to [`Normal`].
	///
	/// [`Normal`]: FontScale::Normal
	pub fn of_style(style: &VisualStyle) -> Self {
		Self::from_font_size(style.font_size, Vec2::ZERO)
	}
}

// ── Fullwidth (> 1em) ───────────────────────────────────────────────────────

/// The fullwidth (ideographic) space, two cells wide. Used as the inter-word
/// separator for fullwidth text so the gap scales with the glyphs.
pub(super) const FULLWIDTH_SPACE: char = '\u{3000}';

/// Map ASCII printable characters to their Unicode fullwidth twins, so they
/// render at double width through the existing wide-character path. Space maps
/// to the [`FULLWIDTH_SPACE`] (also two cells); other characters pass through
/// unchanged.
pub(super) fn to_fullwidth(text: &str) -> String {
	text.chars().map(fullwidth_char).collect()
}

/// Fullwidth twin of a single character (see [`to_fullwidth`]).
fn fullwidth_char(ch: char) -> char {
	match ch {
		' ' => FULLWIDTH_SPACE,
		// ASCII '!'..='~' sit 0xFEE0 below their fullwidth forms 'Ａ'..
		'\u{21}'..='\u{7e}' => char::from_u32(ch as u32 + 0xFEE0).unwrap_or(ch),
		_ => ch,
	}
}

/// Inverse of [`to_fullwidth`]: fold fullwidth glyphs back to ASCII, leaving all
/// other characters unchanged. Used by tests to search a charcell render for its
/// plain text content.
pub fn from_fullwidth(text: &str) -> String {
	text.chars().map(from_fullwidth_char).collect()
}

/// ASCII twin of a single fullwidth character (see [`from_fullwidth`]).
fn from_fullwidth_char(ch: char) -> char {
	match ch {
		FULLWIDTH_SPACE => ' ',
		// fullwidth 'Ａ'..='～' sit 0xFEE0 above their ASCII forms '!'..='~'
		'\u{ff01}'..='\u{ff5e}' => {
			char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch)
		}
		_ => ch,
	}
}

// ── Block font (> 2em) ──────────────────────────────────────────────────────

/// Word-space width used when `font.txt` declares no `space` directive.
const DEFAULT_WORD_SPACE: u32 = 1;
/// Blank rows inserted between wrapped block-font lines.
const BLOCK_LINE_GAP: u32 = 1;
/// Advance (and fallback fullwidth width) for a character with no block glyph.
const BLOCK_FALLBACK_WIDTH: u32 = 2;

/// One glyph of the [`BlockFont`]: `height` rows each `width` columns wide.
struct Glyph {
	width: u32,
	rows: Vec<SmolStr>,
}

/// The multi-row box-drawing font, parsed from `font.txt`.
///
/// Glyphs are keyed by their uppercase form, so `a` and `A` share a glyph; the
/// case selects the pipe style at paint time (uppercase -> double pipe).
pub(super) struct BlockFont {
	height: u32,
	/// Columns advanced for a space between words, from the `space` directive.
	word_space: u32,
	glyphs: HashMap<char, Glyph>,
}

/// The parsed `font.txt`, built once on first use.
static BLOCK_FONT: LazyLock<BlockFont> =
	LazyLock::new(|| parse_block_font(include_str!("font.txt")));

/// The shared block font.
pub(super) fn block_font() -> &'static BlockFont { &BLOCK_FONT }

/// Parse a `font.txt` source into a [`BlockFont`].
///
/// `height N` (first) sets the glyph row count and `space N` the word-space
/// width; each `glyph <CHAR>` header is followed by exactly `height` row lines,
/// read verbatim so blank rows survive. Rows are right-padded to the widest row,
/// which sets the glyph's width.
fn parse_block_font(src: &str) -> BlockFont {
	let mut height = 3usize;
	let mut word_space = DEFAULT_WORD_SPACE;
	let mut glyphs = HashMap::<char, Glyph>::default();
	let mut lines = src.lines();
	while let Some(line) = lines.next() {
		let trimmed = line.trim();
		if trimmed.is_empty() || trimmed.starts_with('#') {
			continue;
		}
		if let Some(rest) = trimmed.strip_prefix("height ") {
			height = rest.trim().parse().unwrap_or(3);
			continue;
		}
		if let Some(rest) = trimmed.strip_prefix("space ") {
			word_space = rest.trim().parse().unwrap_or(DEFAULT_WORD_SPACE);
			continue;
		}
		let Some(rest) = line.strip_prefix("glyph ") else {
			continue;
		};
		let Some(ch) = rest.chars().next() else {
			continue;
		};
		// the next `height` lines are this glyph's rows, taken verbatim
		let rows: Vec<&str> =
			(0..height).map(|_| lines.next().unwrap_or("")).collect();
		let width =
			rows.iter().map(|row| display_width(row)).max().unwrap_or(0);
		let rows = rows.iter().map(|row| pad_to_width(row, width)).collect();
		glyphs.insert(ch, Glyph {
			width: width as u32,
			rows,
		});
	}
	BlockFont {
		height: height as u32,
		word_space,
		glyphs,
	}
}

/// Right-pad `row` with spaces to `width` display columns.
fn pad_to_width(row: &str, width: usize) -> SmolStr {
	let pad = width.saturating_sub(display_width(row));
	if pad == 0 {
		SmolStr::from(row)
	} else {
		let mut out = String::with_capacity(row.len() + pad);
		out.push_str(row);
		out.extend(core::iter::repeat(' ').take(pad));
		SmolStr::from(out)
	}
}

impl BlockFont {
	/// Glyph for `ch`, keyed by its uppercase form (`a` and `A` share a glyph).
	fn glyph(&self, ch: char) -> Option<&Glyph> {
		self.glyphs.get(&ch.to_ascii_uppercase())
	}

	/// Columns a character advances: the configured word space, its glyph width,
	/// or the fallback width when the font has no glyph for it.
	fn advance(&self, ch: char) -> u32 {
		match ch {
			' ' => self.word_space,
			_ => self.glyph(ch).map_or(BLOCK_FALLBACK_WIDTH, |g| g.width),
		}
	}

	/// Total advance of an already-wrapped line.
	fn line_width(&self, line: &str) -> u32 {
		line.chars().map(|ch| self.advance(ch)).sum()
	}
}

/// Map a single-pipe box-drawing character to its double-pipe twin, the
/// uppercase variant (per `font.md`). Non-box characters pass through.
fn to_double_pipe(ch: char) -> char {
	match ch {
		'┌' => '╔',
		'─' => '═',
		'┐' => '╗',
		'│' => '║',
		'└' => '╚',
		'┘' => '╝',
		'├' => '╠',
		'┤' => '╣',
		'┬' => '╦',
		'┴' => '╩',
		'┼' => '╬',
		other => other,
	}
}

/// Greedy word-wrap of `text` to `max_w` columns using block-glyph advances.
/// Splits on `\n`, breaks on word boundaries, and hard-breaks an overlong word.
fn block_wrap(font: &BlockFont, text: &str, max_w: u32) -> Vec<String> {
	if max_w == 0 {
		return vec![];
	}
	let mut lines = Vec::new();
	for para in text.split('\n') {
		let mut cur = String::new();
		let mut cur_w = 0u32;
		for word in para.split_whitespace() {
			let word_w = word.chars().map(|ch| font.advance(ch)).sum::<u32>();
			let space_w = if cur.is_empty() { 0 } else { font.word_space };
			// wrap before a word that would overflow the current line
			if !cur.is_empty() && cur_w + space_w + word_w > max_w {
				lines.push(core::mem::take(&mut cur));
				cur_w = 0;
			}
			if !cur.is_empty() {
				cur.push(' ');
				cur_w += font.word_space;
			}
			if word_w > max_w {
				// hard-break a word wider than the whole line
				for ch in word.chars() {
					let advance = font.advance(ch);
					if !cur.is_empty() && cur_w + advance > max_w {
						lines.push(core::mem::take(&mut cur));
						cur_w = 0;
					}
					cur.push(ch);
					cur_w += advance;
				}
			} else {
				cur.push_str(word);
				cur_w += word_w;
			}
		}
		lines.push(cur);
	}
	lines
}

/// Row stride between wrapped block-font lines (glyph height plus the gap).
fn block_line_stride(font: &BlockFont) -> u32 { font.height + BLOCK_LINE_GAP }

/// Measure `text` in the block font wrapped to `max_w`, returning
/// `(max_line_width, total_rows)`. Empty text reserves no rows.
pub(super) fn measure_block_text(text: &str, max_w: u32) -> UVec2 {
	if text.trim().is_empty() {
		return UVec2::ZERO;
	}
	let font = block_font();
	let lines = block_wrap(font, text, max_w);
	if lines.is_empty() {
		return UVec2::ZERO;
	}
	let width = lines
		.iter()
		.map(|line| font.line_width(line))
		.max()
		.unwrap_or(0);
	let rows = lines.len() as u32 * font.height
		+ (lines.len() as u32).saturating_sub(1) * BLOCK_LINE_GAP;
	UVec2::new(width, rows)
}

/// Paint `text` in the block font into `content_rect`, wrapping to its width and
/// aligning each line. Uppercase letters use the double-pipe variant.
pub(super) fn paint_block_text(
	text: &str,
	content_rect: IRect,
	style: &VisualStyle,
	align: TextAlign,
	entity: Entity,
	buffer: &mut impl AsBuffer,
	clip: Clip,
) {
	if text.trim().is_empty() {
		return;
	}
	let font = block_font();
	let width = content_rect.width().max(0) as u32;
	let lines = block_wrap(font, text, width);
	let stride = block_line_stride(font) as i32;
	let mut y = content_rect.min.y;
	for line in &lines {
		if y >= content_rect.max.y {
			break;
		}
		let mut x = content_rect.min.x
			+ align_offset(font.line_width(line), width, align) as i32;
		for ch in line.chars() {
			paint_glyph(
				font,
				ch,
				IVec2::new(x, y),
				style,
				entity,
				buffer,
				clip,
			);
			x += font.advance(ch) as i32;
		}
		y += stride;
	}
}

/// Paint one block glyph at `origin`. A space advances only; a missing glyph
/// falls back to its fullwidth twin on the middle row.
fn paint_glyph(
	font: &BlockFont,
	ch: char,
	origin: IVec2,
	style: &VisualStyle,
	entity: Entity,
	buffer: &mut impl AsBuffer,
	clip: Clip,
) {
	if ch == ' ' {
		return;
	}
	let Some(glyph) = font.glyph(ch) else {
		// no glyph: drop the fullwidth twin on the middle row so nothing is lost
		let mid = origin.y + (font.height as i32) / 2;
		buffer.write_text(
			IVec2::new(origin.x, mid),
			&fullwidth_char(ch).to_string(),
			style.clone(),
			entity,
			clip,
		);
		return;
	};
	let double = ch.is_ascii_uppercase();
	for (row, line) in glyph.rows.iter().enumerate() {
		let cased: String = if double {
			line.chars().map(to_double_pipe).collect()
		} else {
			line.to_string()
		};
		buffer.write_text(
			IVec2::new(origin.x, origin.y + row as i32),
			&cased,
			style.clone(),
			entity,
			clip,
		);
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A throwaway 3-row font: `A` (width 3), narrow `I` (width 1).
	const TEST_SRC: &str = "height 3
glyph A
┌─┐
├─┤
┴ ┴
glyph I
┬
│
┴
";

	#[beet_core::test]
	fn scale_thresholds() {
		let vp = Vec2::new(80., 24.);
		FontScale::from_font_size(Length::Rem(1.0), vp)
			.xpect_eq(FontScale::Normal);
		FontScale::from_font_size(Length::Rem(1.5), vp)
			.xpect_eq(FontScale::Wide);
		// the > thresholds are strict: exactly 2em is still wide, not block
		FontScale::from_font_size(Length::Rem(2.0), vp)
			.xpect_eq(FontScale::Wide);
		FontScale::from_font_size(Length::Rem(2.5), vp)
			.xpect_eq(FontScale::Block);
	}

	#[beet_core::test]
	fn fullwidth_remaps_ascii_and_space() {
		// 'A' -> 'Ａ' (U+FF21), ' ' -> ideographic space (U+3000)
		to_fullwidth("A B").xpect_eq("Ａ　Ｂ".to_string());
		// each fullwidth glyph measures two columns
		display_width(&to_fullwidth("AB")).xpect_eq(4);
		// `from_fullwidth` is the inverse round-trip
		from_fullwidth(&to_fullwidth("A B!")).xpect_eq("A B!".to_string());
	}

	#[beet_core::test]
	fn parses_glyph_widths_and_padding() {
		let font = parse_block_font(TEST_SRC);
		font.height.xpect_eq(3);
		font.glyph('A').unwrap().width.xpect_eq(3);
		// lowercase resolves to the same glyph
		font.glyph('a').unwrap().width.xpect_eq(3);
		font.glyph('I').unwrap().width.xpect_eq(1);
		// shorter rows are right-padded to the glyph width
		font.glyph('A')
			.unwrap()
			.rows
			.iter()
			.all(|row| display_width(row) == 3)
			.xpect_true();
	}

	#[beet_core::test]
	fn wraps_by_glyph_advance() {
		let font = parse_block_font(TEST_SRC);
		// "AA AA" at width 7: "AA"=6, +space+"AA" = 6+1+6 > 7, so wraps
		block_wrap(&font, "AA AA", 7)
			.xpect_eq(vec!["AA".to_string(), "AA".to_string()]);
		// both fit at width 13 (6 + 1 + 6)
		block_wrap(&font, "AA AA", 13).xpect_eq(vec!["AA AA".to_string()]);
	}

	#[beet_core::test]
	fn line_width_sums_advances() {
		let font = parse_block_font(TEST_SRC);
		// "AI" = 3 + 1, "A A" = 3 + 1(space) + 3
		font.line_width("AI").xpect_eq(4);
		font.line_width("A A").xpect_eq(7);
	}

	#[beet_core::test]
	fn space_directive_sets_word_space() {
		// no directive -> the 1-cell default
		parse_block_font(TEST_SRC).word_space.xpect_eq(1);
		// `space N` widens the inter-word gap: "A A" = 3 + 3(space) + 3
		let wide =
			parse_block_font("height 3\nspace 3\nglyph A\n┌─┐\n├─┤\n┴ ┴\n");
		wide.word_space.xpect_eq(3);
		wide.line_width("A A").xpect_eq(9);
	}
}
