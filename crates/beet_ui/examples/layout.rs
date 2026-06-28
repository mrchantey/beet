// Under `cfg(test)` the demo `main` and its `setup_*` helpers are stripped, so
// they read as dead code / unused imports; that is expected for the test build.
#![cfg_attr(test, allow(dead_code, unused_imports))]
use beet_core::prelude::*;
use beet_ui::prelude::style::AlignItems;
use beet_ui::prelude::style::Display;
use beet_ui::prelude::style::FlexWrap;
use beet_ui::prelude::style::FontStyle;
use beet_ui::prelude::style::FontWeight;
use beet_ui::prelude::style::JustifyContent;
use beet_ui::prelude::style::Visibility;
use beet_ui::prelude::style::*;
use beet_ui::prelude::*;

#[cfg(not(test))]
fn main() {
	let size = terminal_ext::size();
	cross_log!("=== Beet Layout Engine Demo ({}×{}) ===\n", size.x, size.y);

	let mut overflow = 0;
	overflow += render("JustifyContent::Start", setup_justify_start);
	overflow += render("JustifyContent::Center", setup_justify_center);
	overflow += render("JustifyContent::End", setup_justify_end);
	overflow +=
		render("JustifyContent::SpaceBetween", setup_justify_space_between);
	overflow +=
		render("JustifyContent::SpaceAround", setup_justify_space_around);
	overflow +=
		render("JustifyContent::SpaceEvenly", setup_justify_space_evenly);

	overflow += render("AlignItems::Start", setup_align_start);
	overflow += render("AlignItems::Center", setup_align_center);
	overflow += render("AlignItems::End", setup_align_end);
	overflow += render("AlignItems::Stretch", setup_align_stretch);

	overflow += render("Row and Column Gaps", setup_gaps);
	overflow += render("Flex Grow", setup_flex_grow);
	overflow += render("No Wrap", setup_no_wrap);
	overflow += render("Wrap", setup_wrap);
	overflow += render("Nested Rows and Columns", setup_nested);

	overflow += render("Margin Only", setup_margin_only);
	overflow += render("Border Only", setup_border_only);
	overflow += render("Padding Only", setup_padding_only);
	overflow += render("Margin + Border + Padding", setup_all_spacing);

	// Style demos (ANSI color output)
	overflow += render("Foreground Color", setup_foreground_color);
	overflow += render("Background Color", setup_background_color);
	overflow += render("Border Color (per-side)", setup_border_color);
	overflow += render("Text Formatting", setup_text_formatting);
	overflow += render("Text Italic", setup_text_italic);
	overflow += render("Text Blink", setup_text_blink);
	overflow += render("Text Hidden", setup_text_hidden);
	overflow += render("Inline Layout", setup_inline_basic);
	overflow += render("Inline Wrap", setup_inline_wrap);
	overflow += render("Text Align", setup_text_align);
	overflow += render("Wide Characters (CJK)", setup_wide_chars);
	overflow += render("Font Size: Wide (>1em)", setup_font_wide);
	overflow += render("Font Size: Block (>2em)", setup_font_block);

	// Render width must never exceed the measured terminal width, otherwise the
	// terminal soft-wraps and every box appears one column too wide.
	cross_log!("");
	match overflow {
		0 => {
			cross_log!(
				"✓ all lines render within the {}-column terminal",
				size.x
			)
		}
		n => {
			cross_log!("✗ {n} line(s) exceeded the {}-column terminal", size.x)
		}
	}
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Render a demo and return the number of lines wider than the terminal.
fn render<B: Bundle>(name: &str, setup: fn() -> B) -> usize {
	let width = terminal_ext::size().x as usize;
	let out = Buffer::render_oneshot(setup()).trim_lines();
	let over = out
		.lines()
		.filter(|line| display_width(line) > width)
		.count();
	cross_log!("\n{name}: \n{out}");
	if over > 0 {
		cross_log!("  ⚠ {over} line(s) exceed the {width}-column width");
	}
	over
}

fn bordered() -> BoxStyle {
	BoxStyle::default().with_border(Spacing::all(Length::Rem(1.)))
}

fn margin() -> BoxStyle {
	BoxStyle::default().with_margin(Spacing::all(Length::Rem(1.)))
}

// ── JustifyContent ────────────────────────────────────────────────────────────

fn setup_justify_start() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.justify_content(JustifyContent::Start)
			.column_gap(Length::Rem(1.)),
		children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
			(rsx! {"C"}, bordered()),
		],
	)
}

fn setup_justify_center() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.justify_content(JustifyContent::Center)
			.column_gap(Length::Rem(1.)),
		children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
			(rsx! {"C"}, bordered()),
		],
	)
}

fn setup_justify_end() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.justify_content(JustifyContent::End)
			.column_gap(Length::Rem(1.)),
		children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
			(rsx! {"C"}, bordered()),
		],
	)
}

fn setup_justify_space_between() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.justify_content(JustifyContent::SpaceBetween)
			.column_gap(Length::Rem(1.)),
		children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
			(rsx! {"C"}, bordered()),
		],
	)
}

fn setup_justify_space_around() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.justify_content(JustifyContent::SpaceAround)
			.column_gap(Length::Rem(1.)),
		children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
			(rsx! {"C"}, bordered()),
		],
	)
}

fn setup_justify_space_evenly() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.justify_content(JustifyContent::SpaceEvenly)
			.column_gap(Length::Rem(1.)),
		children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
			(rsx! {"C"}, bordered()),
		],
	)
}

// ── AlignItems ────────────────────────────────────────────────────────────────

fn setup_align_start() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.align_items(AlignItems::Start)
			.column_gap(Length::Rem(1.)),
		children![
			(
				LayoutStyle::flex_col(),
				children![
					(rsx! {"Very"}, bordered()),
					(rsx! {"Tall"}, bordered()),
					(rsx! {"Item"}, bordered()),
				],
				bordered()
			),
			(rsx! {"Short"}, bordered()),
			(rsx! {"B"}, bordered()),
		],
	)
}

fn setup_align_center() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.align_items(AlignItems::Center)
			.column_gap(Length::Rem(1.)),
		children![
			(
				LayoutStyle::flex_col(),
				children![
					(rsx! {"Very"}, bordered()),
					(rsx! {"Tall"}, bordered()),
					(rsx! {"Item"}, bordered()),
				],
				bordered()
			),
			(rsx! {"Short"}, bordered()),
			(rsx! {"B"}, bordered()),
		],
	)
}

fn setup_align_end() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.align_items(AlignItems::End)
			.column_gap(Length::Rem(1.)),
		children![
			(
				LayoutStyle::flex_col(),
				children![
					(rsx! {"Very"}, bordered()),
					(rsx! {"Tall"}, bordered()),
					(rsx! {"Item"}, bordered()),
				],
				bordered()
			),
			(rsx! {"Short"}, bordered()),
			(rsx! {"B"}, bordered()),
		],
	)
}

fn setup_align_stretch() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.align_items(AlignItems::Stretch)
			.column_gap(Length::Rem(1.)),
		children![
			(
				LayoutStyle::flex_col(),
				children![
					(rsx! {"Very"}, bordered()),
					(rsx! {"Tall"}, bordered()),
					(rsx! {"Item"}, bordered()),
				],
				bordered()
			),
			(rsx! {"Short"}, bordered()),
			(rsx! {"B"}, bordered()),
		],
	)
}

// ── Gaps ──────────────────────────────────────────────────────────────────────

fn setup_gaps() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.wrap(FlexWrap::Wrap)
			.row_gap(Length::Rem(1.))
			.column_gap(Length::Rem(2.)),
		children![
			(rsx! {"1"}, bordered()),
			(rsx! {"2"}, bordered()),
			(rsx! {"3"}, bordered()),
			(rsx! {"4"}, bordered()),
			(rsx! {"5"}, bordered()),
			(rsx! {"6"}, bordered()),
		],
	)
}

// ── Flex Grow ─────────────────────────────────────────────────────────────────

fn setup_flex_grow() -> impl Bundle {
	(
		LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
		children![
			(rsx! {"Fixed"}, bordered()),
			(
				rsx! {"Grow 1"},
				bordered(),
				LayoutStyle::default().with_flex_grow(1)
			),
			(
				rsx! {"Grow 2"},
				bordered(),
				LayoutStyle::default().with_flex_grow(2)
			),
			(rsx! {"Fixed"}, bordered()),
		],
	)
}

// ── Wrapping ──────────────────────────────────────────────────────────────────

fn setup_no_wrap() -> impl Bundle {
	(
		LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
		children![
			(rsx! {"Item 1"}, bordered()),
			(rsx! {"Item 2"}, bordered()),
			(rsx! {"Item 3"}, bordered()),
			(rsx! {"Item 4"}, bordered()),
			(rsx! {"Item 5"}, bordered()),
		],
	)
}

fn setup_wrap() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.wrap(FlexWrap::Wrap)
			.column_gap(Length::Rem(1.))
			.row_gap(Length::Rem(1.)),
		children![
			(rsx! {"Item 1"}, bordered()),
			(rsx! {"Item 2"}, bordered()),
			(rsx! {"Item 3"}, bordered()),
			(rsx! {"Item 4"}, bordered()),
			(rsx! {"Item 5"}, bordered()),
		],
	)
}

// ── Nested Layouts ────────────────────────────────────────────────────────────

fn setup_nested() -> impl Bundle {
	// Each node gets a distinct background so background ordering is visible.
	let header_style = VisualStyle {
		background: Some(Color::srgb(0.2, 0.2, 0.5)),
		foreground: Some(Color::WHITE),
		..default()
	};
	let sidebar_style = VisualStyle {
		background: Some(Color::srgb(0.2, 0.4, 0.2)),
		foreground: Some(Color::WHITE),
		..default()
	};
	let main_style = VisualStyle {
		background: Some(Color::srgb(0.4, 0.2, 0.2)),
		foreground: Some(Color::WHITE),
		..default()
	};
	let footer_style = VisualStyle {
		background: Some(Color::srgb(0.3, 0.3, 0.1)),
		foreground: Some(Color::WHITE),
		..default()
	};
	(LayoutStyle::flex_col().row_gap(Length::Rem(1.)), children![
		(
			LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
			children![
				(rsx! {"Header L"}, bordered(), header_style.clone()),
				(rsx! {"Header R"}, bordered(), header_style.clone()),
			],
			bordered()
		),
		(
			LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
			VisualStyle::default()
				.with_background(palettes::tailwind::EMERALD_900),
			children![
				(rsx! {"Sidebar"}, bordered(), sidebar_style),
				(
					LayoutStyle::flex_col().row_gap(Length::Rem(1.)),
					children![
						(rsx! {"Main"}, bordered(), main_style),
						(rsx! {"Footer"}, bordered(), footer_style),
					],
					bordered()
				),
			],
			bordered()
		),
	])
}

// ── Margins, Borders, Padding ─────────────────────────────────────────────────

fn setup_margin_only() -> impl Bundle {
	(
		LayoutStyle::flex_row().column_gap(Length::Rem(0.)),
		children![
			(rsx! {"A"}, margin()),
			(rsx! {"B"}, margin()),
			(rsx! {"C"}, margin()),
		],
	)
}

fn setup_border_only() -> impl Bundle {
	(
		LayoutStyle::flex_row().column_gap(Length::Rem(0.)),
		children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
			(rsx! {"C"}, bordered()),
		],
	)
}

fn setup_padding_only() -> impl Bundle {
	let style = bordered().with_padding(Spacing::all(Length::Rem(1.)));
	(
		LayoutStyle::flex_row().column_gap(Length::Rem(0.)),
		children![
			(rsx! {"A"}, style.clone()),
			(rsx! {"B"}, style.clone()),
			(rsx! {"C"}, style.clone()),
		],
	)
}

fn setup_all_spacing() -> impl Bundle {
	let style = BoxStyle::default()
		.with_margin(Spacing::all(Length::Rem(1.)))
		.with_border(Spacing::all(Length::Rem(1.)))
		.with_padding(Spacing::all(Length::Rem(1.)));

	(
		LayoutStyle::flex_row().column_gap(Length::Rem(0.)),
		children![
			(rsx! {"A"}, style.clone()),
			(rsx! {"B"}, style.clone()),
			(rsx! {"C"}, style.clone()),
		],
	)
}

// ── Style / Color ─────────────────────────────────────────────────────────────

fn setup_foreground_color() -> impl Bundle {
	(
		LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
		children![
			(rsx! { "Red" }, bordered(), VisualStyle {
				foreground: Some(Color::srgb(1., 0., 0.)),
				..default()
			},),
			(rsx! { "Green" }, bordered(), VisualStyle {
				foreground: Some(Color::srgb(0., 0.8, 0.)),
				..default()
			},),
			(rsx! { "Blue" }, bordered(), VisualStyle {
				foreground: Some(Color::srgb(0.2, 0.4, 1.)),
				..default()
			},),
		],
	)
}

fn setup_background_color() -> impl Bundle {
	(
		LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
		children![
			(
				rsx! { "A" },
				bordered().with_padding(Spacing::all(Length::Rem(0.5))),
				VisualStyle {
					background: Some(Color::srgb(0.5, 0., 0.5)),
					foreground: Some(Color::WHITE),
					..default()
				},
			),
			(
				rsx! { "B" },
				bordered().with_padding(Spacing::all(Length::Rem(0.5))),
				VisualStyle {
					background: Some(Color::srgb(0., 0.4, 0.6)),
					foreground: Some(Color::WHITE),
					..default()
				},
			),
		],
	)
}

fn setup_border_color() -> impl Bundle {
	// Each node gets per-side border colors: top=red, bottom=blue, left=green, right=yellow
	(
		LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
		children![(rsx! { "Box" }, BoxStyle {
			border: Spacing::all(Length::Rem(1.)),
			border_top: Some(Color::srgb(1., 0., 0.)),
			border_bottom: Some(Color::srgb(0., 0.4, 1.)),
			border_left: Some(Color::srgb(0., 0.8, 0.)),
			border_right: Some(Color::srgb(1., 0.8, 0.)),
			..default()
		},),],
	)
}

fn setup_text_formatting() -> impl Bundle {
	(
		LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
		children![
			(rsx! { "Underline" }, bordered(), VisualStyle {
				decoration_line: DecorationLine::underline(),
				..default()
			},),
			(rsx! { "Strike" }, bordered(), VisualStyle {
				decoration_line: DecorationLine::line_through(),
				..default()
			},),
			(rsx! { "Bold" }, bordered(), VisualStyle {
				font_weight: FontWeight::Bold,
				..default()
			},),
		],
	)
}

fn setup_text_italic() -> impl Bundle {
	(
		LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
		children![
			(rsx! { "Italic" }, bordered(), VisualStyle {
				font_style: FontStyle::Italic,
				..default()
			}),
			(rsx! { "Bold+Italic" }, bordered(), VisualStyle {
				font_weight: FontWeight::Bold,
				font_style: FontStyle::Italic,
				..default()
			}),
		],
	)
}

fn setup_text_blink() -> impl Bundle {
	(
		LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
		children![
			(rsx! { "Blink" }, bordered(), VisualStyle {
				blink: BlinkStyle::Blink,
				..default()
			}),
			(rsx! { "RapidBlink" }, bordered(), VisualStyle {
				blink: BlinkStyle::RapidBlink,
				..default()
			}),
		],
	)
}

fn setup_text_hidden() -> impl Bundle {
	(
		LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
		children![
			(rsx! { "Visible" }, bordered(), VisualStyle {
				visibility: Visibility::Visible,
				..default()
			}),
			(rsx! { "Hidden" }, bordered(), VisualStyle {
				visibility: Visibility::Hidden,
				..default()
			}),
		],
	)
}

fn setup_inline_basic() -> impl Bundle {
	(
		LayoutStyle {
			display: Display::Inline,
			..default()
		},
		children![
			rsx! { "Hello" },
			rsx! { " " },
			rsx! { "World" },
			rsx! { " " },
			rsx! { "Inline!" },
		],
	)
}

fn setup_inline_wrap() -> impl Bundle {
	// Items side-by-side, wrapping when overflow
	(
		LayoutStyle {
			display: Display::Inline,
			..default()
		},
		children![
			rsx! { "A Very Long Sentence " },
			rsx! { "A Very Long Sentence " },
			rsx! { "A Very Long Sentence " },
			rsx! { "A Very Long Sentence " },
			rsx! { "A Very Long Sentence " },
			rsx! { "A Very Long Sentence " },
		],
	)
}

fn setup_wide_chars() -> impl Bundle {
	// CJK and fullwidth characters occupy 2 terminal columns each
	(
		LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
		children![
			(rsx! { "中文" }, bordered(), VisualStyle {
				foreground: Some(Color::srgb(0.9, 0.5, 0.1)),
				..default()
			}),
			(rsx! { "日本語" }, bordered(), VisualStyle {
				foreground: Some(Color::srgb(0.1, 0.7, 0.9)),
				..default()
			}),
			(rsx! { "ＡＢＣ" }, bordered(), VisualStyle {
				foreground: Some(Color::srgb(0.5, 0.9, 0.4)),
				..default()
			}),
		],
	)
}

// ── Font scaling ────────────────────────────────────────────────────────────

/// A `font-size` above 1em renders text as fullwidth glyphs (2 columns each).
fn setup_font_wide() -> impl Bundle {
	let wide = VisualStyle::default().with_font_size(Length::Rem(1.5));
	(
		LayoutStyle::flex_row().column_gap(Length::Rem(2.)),
		children![
			(rsx! { "Title" }, wide.clone()),
			(rsx! { "Heading 42" }, wide),
		],
	)
}

/// A `font-size` above 2em renders the box-drawing block font (3 rows tall),
/// capitals in the double-pipe variant.
fn setup_font_block() -> impl Bundle {
	(LayoutStyle::flex_col().row_gap(Length::Rem(1.)), children![
		(
			rsx! { "Beet" },
			VisualStyle::default()
				.with_font_size(Length::Rem(3.))
				.with_foreground(Color::srgb(0.5, 0.9, 0.4)),
		),
		(
			rsx! { "ui 2024" },
			VisualStyle::default().with_font_size(Length::Rem(2.5))
		),
	])
}

fn setup_text_align() -> impl Bundle {
	let item_styles = (
		BoxStyle::default()
			.with_padding(Spacing::all(Length::Rem(2.)))
			.with_border(Spacing::all(Length::Rem(1.))),
		LayoutStyle::default().with_flex_grow(1),
	);

	(
		LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
		children![
			(rsx! { "Left" }, item_styles.clone(), VisualStyle {
				text_align: TextAlign::Left,
				..default()
			},),
			(rsx! { "Center" }, item_styles.clone(), VisualStyle {
				text_align: TextAlign::Center,
				..default()
			},),
			(rsx! { "Right" }, item_styles, VisualStyle {
				text_align: TextAlign::Right,
				..default()
			},),
		],
	)
}

// ── Layout measurement tests ────────────────────────────────────────────────
//
// These run under the beet test runner via `cargo test --example layout
// --features tui`. They pin the measurement contract the layout engine must
// uphold: a text node's width is its display-column count, wrapping breaks on
// word boundaries within the content width, and block children stack their
// rows. They also lock in the wide/fullwidth and box-drawing font behaviour
// driven by `font-size` (see `VisualStyle::font_size`).

#[cfg(test)]
beet_core::test_main!();

#[cfg(test)]
mod test {
	use super::*;
	use beet_core::prelude::*;

	/// Render `bundle` into a `cols × rows` buffer, trimmed to its plain lines.
	fn lines(cols: u32, rows: u32, bundle: impl Bundle) -> Vec<String> {
		Buffer::render_oneshot_plain_sized(UVec2::new(cols, rows), bundle)
			.trim_lines()
			.lines()
			.map(|line| line.trim_end().to_string())
			.collect()
	}

	/// Widest rendered line, in display columns.
	fn max_width(lines: &[String]) -> usize {
		lines
			.iter()
			.map(|line| display_width(line))
			.max()
			.unwrap_or(0)
	}

	#[beet_core::test]
	fn text_width_matches_glyph_count() {
		// a short word measures its own column count, on a single row
		let out = lines(20, 3, rsx! { <p>"Hello"</p> });
		out.len().xpect_eq(1);
		max_width(&out).xpect_eq(5);
	}

	#[beet_core::test]
	fn paragraph_wraps_to_expected_rows() {
		// four words in a 7-column content box wrap on word boundaries
		lines(7, 6, rsx! { <p>"one two three four"</p> }).xpect_eq(vec![
			"one two".to_string(),
			"three".to_string(),
			"four".to_string(),
		]);
	}

	#[beet_core::test]
	fn wrapped_text_never_exceeds_width() {
		// no wrapped row may overflow the content width
		let out =
			lines(12, 8, rsx! { <p>"alpha beta gamma delta epsilon zeta"</p> });
		(max_width(&out) <= 12).xpect_true();
		(out.len() >= 3).xpect_true();
	}

	#[beet_core::test]
	fn block_children_stack_vertically() {
		// three stacked paragraphs occupy three content rows
		let out =
			lines(20, 10, rsx! { <div><p>"A"</p><p>"B"</p><p>"C"</p></div> });
		out.iter()
			.filter(|line| !line.is_empty())
			.count()
			.xpect_eq(3);
	}

	#[beet_core::test]
	fn wide_cjk_doubles_measured_width() {
		// fullwidth/CJK glyphs occupy two columns each
		let out = lines(20, 3, rsx! { <p>"中文"</p> });
		max_width(&out).xpect_eq(4);
	}

	// ── Font scaling (font-size driven) ─────────────────────────────────────

	/// A leaf carrying a `font-size`. A bare value node has no `Element`, so the
	/// style cascade leaves the hand-attached `VisualStyle` intact.
	fn sized(text: &'static str, rem: f32) -> impl Bundle {
		(
			rsx! { {text} },
			VisualStyle::default().with_font_size(Length::Rem(rem)),
		)
	}

	#[beet_core::test]
	fn normal_font_renders_plain_ascii() {
		// <= 1em keeps the unscaled single-cell glyphs
		lines(20, 4, sized("AB", 1.0)).xpect_eq(vec!["AB".to_string()]);
	}

	#[beet_core::test]
	fn wide_font_doubles_width() {
		// (1em, 2em] renders fullwidth: two columns per glyph, one row
		let out = lines(20, 4, sized("AB", 1.5));
		out.len().xpect_eq(1);
		max_width(&out).xpect_eq(4);
		out[0].as_str().xpect_contains("Ａ");
	}

	#[beet_core::test]
	fn block_font_is_three_rows_tall() {
		// > 2em renders the box-drawing block font, three rows tall, with
		// uppercase letters drawn in the double-pipe variant
		let out = lines(40, 8, sized("AB", 2.5));
		out.len().xpect_eq(3);
		out.join("\n").as_str().xpect_contains("╔");
	}

	#[beet_core::test]
	fn block_font_lowercase_uses_single_pipe() {
		// lowercase keeps the single-pipe glyph; only capitals double the pipes
		let out = lines(40, 8, sized("ab", 2.5));
		out.len().xpect_eq(3);
		let joined = out.join("\n");
		joined.as_str().xpect_contains("┌");
		joined.as_str().xnot().xpect_contains("╔");
	}

	#[beet_core::test]
	fn block_font_wraps_within_width() {
		// a block heading wider than the box wraps onto more three-row lines
		let out = lines(18, 12, sized("ALPHA BETA GAMMA", 3.0));
		// at least two wrapped lines (>= 6 rows incl. the inter-line gap)
		(out.len() >= 6).xpect_true();
		(max_width(&out) <= 18).xpect_true();
	}

	/// Visual snapshot of the box-drawing block font (uppercase double-pipe).
	#[beet_core::test]
	fn block_font_snapshot() {
		Buffer::render_oneshot_plain_sized(
			UVec2::new(60, 6),
			sized("Beet", 3.0),
		)
		.trim_lines()
		.xpect_snapshot();
	}
}
