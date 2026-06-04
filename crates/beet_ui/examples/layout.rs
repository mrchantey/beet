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
use beet_ui::*;

fn main() {
	let size = terminal_ext::size();
	println!("=== Beet Layout Engine Demo ({}×{}) ===\n", size.x, size.y);

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

	// Render width must never exceed the measured terminal width, otherwise the
	// terminal soft-wraps and every box appears one column too wide.
	println!();
	match overflow {
		0 => {
			println!("✓ all lines render within the {}-column terminal", size.x)
		}
		n => println!("✗ {n} line(s) exceeded the {}-column terminal", size.x),
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
	println!("\n{name}: \n{out}");
	if over > 0 {
		println!("  ⚠ {over} line(s) exceed the {width}-column width");
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
			.column_gap(1),
		children![
			(rsx_direct! {"A"}, bordered()),
			(rsx_direct! {"B"}, bordered()),
			(rsx_direct! {"C"}, bordered()),
		],
	)
}

fn setup_justify_center() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.justify_content(JustifyContent::Center)
			.column_gap(1),
		children![
			(rsx_direct! {"A"}, bordered()),
			(rsx_direct! {"B"}, bordered()),
			(rsx_direct! {"C"}, bordered()),
		],
	)
}

fn setup_justify_end() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.justify_content(JustifyContent::End)
			.column_gap(1),
		children![
			(rsx_direct! {"A"}, bordered()),
			(rsx_direct! {"B"}, bordered()),
			(rsx_direct! {"C"}, bordered()),
		],
	)
}

fn setup_justify_space_between() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.justify_content(JustifyContent::SpaceBetween)
			.column_gap(1),
		children![
			(rsx_direct! {"A"}, bordered()),
			(rsx_direct! {"B"}, bordered()),
			(rsx_direct! {"C"}, bordered()),
		],
	)
}

fn setup_justify_space_around() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.justify_content(JustifyContent::SpaceAround)
			.column_gap(1),
		children![
			(rsx_direct! {"A"}, bordered()),
			(rsx_direct! {"B"}, bordered()),
			(rsx_direct! {"C"}, bordered()),
		],
	)
}

fn setup_justify_space_evenly() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.justify_content(JustifyContent::SpaceEvenly)
			.column_gap(1),
		children![
			(rsx_direct! {"A"}, bordered()),
			(rsx_direct! {"B"}, bordered()),
			(rsx_direct! {"C"}, bordered()),
		],
	)
}

// ── AlignItems ────────────────────────────────────────────────────────────────

fn setup_align_start() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.align_items(AlignItems::Start)
			.column_gap(1),
		children![
			(
				LayoutStyle::flex_col(),
				children![
					(rsx_direct! {"Very"}, bordered()),
					(rsx_direct! {"Tall"}, bordered()),
					(rsx_direct! {"Item"}, bordered()),
				],
				bordered()
			),
			(rsx_direct! {"Short"}, bordered()),
			(rsx_direct! {"B"}, bordered()),
		],
	)
}

fn setup_align_center() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.align_items(AlignItems::Center)
			.column_gap(1),
		children![
			(
				LayoutStyle::flex_col(),
				children![
					(rsx_direct! {"Very"}, bordered()),
					(rsx_direct! {"Tall"}, bordered()),
					(rsx_direct! {"Item"}, bordered()),
				],
				bordered()
			),
			(rsx_direct! {"Short"}, bordered()),
			(rsx_direct! {"B"}, bordered()),
		],
	)
}

fn setup_align_end() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.align_items(AlignItems::End)
			.column_gap(1),
		children![
			(
				LayoutStyle::flex_col(),
				children![
					(rsx_direct! {"Very"}, bordered()),
					(rsx_direct! {"Tall"}, bordered()),
					(rsx_direct! {"Item"}, bordered()),
				],
				bordered()
			),
			(rsx_direct! {"Short"}, bordered()),
			(rsx_direct! {"B"}, bordered()),
		],
	)
}

fn setup_align_stretch() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.align_items(AlignItems::Stretch)
			.column_gap(1),
		children![
			(
				LayoutStyle::flex_col(),
				children![
					(rsx_direct! {"Very"}, bordered()),
					(rsx_direct! {"Tall"}, bordered()),
					(rsx_direct! {"Item"}, bordered()),
				],
				bordered()
			),
			(rsx_direct! {"Short"}, bordered()),
			(rsx_direct! {"B"}, bordered()),
		],
	)
}

// ── Gaps ──────────────────────────────────────────────────────────────────────

fn setup_gaps() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.wrap(FlexWrap::Wrap)
			.row_gap(1)
			.column_gap(2),
		children![
			(rsx_direct! {"1"}, bordered()),
			(rsx_direct! {"2"}, bordered()),
			(rsx_direct! {"3"}, bordered()),
			(rsx_direct! {"4"}, bordered()),
			(rsx_direct! {"5"}, bordered()),
			(rsx_direct! {"6"}, bordered()),
		],
	)
}

// ── Flex Grow ─────────────────────────────────────────────────────────────────

fn setup_flex_grow() -> impl Bundle {
	(LayoutStyle::flex_row().column_gap(1), children![
		(rsx_direct! {"Fixed"}, bordered()),
		(
			rsx_direct! {"Grow 1"},
			bordered(),
			LayoutStyle::default().with_flex_grow(1)
		),
		(
			rsx_direct! {"Grow 2"},
			bordered(),
			LayoutStyle::default().with_flex_grow(2)
		),
		(rsx_direct! {"Fixed"}, bordered()),
	])
}

// ── Wrapping ──────────────────────────────────────────────────────────────────

fn setup_no_wrap() -> impl Bundle {
	(LayoutStyle::flex_row().column_gap(1), children![
		(rsx_direct! {"Item 1"}, bordered()),
		(rsx_direct! {"Item 2"}, bordered()),
		(rsx_direct! {"Item 3"}, bordered()),
		(rsx_direct! {"Item 4"}, bordered()),
		(rsx_direct! {"Item 5"}, bordered()),
	])
}

fn setup_wrap() -> impl Bundle {
	(
		LayoutStyle::flex_row()
			.wrap(FlexWrap::Wrap)
			.column_gap(1)
			.row_gap(1),
		children![
			(rsx_direct! {"Item 1"}, bordered()),
			(rsx_direct! {"Item 2"}, bordered()),
			(rsx_direct! {"Item 3"}, bordered()),
			(rsx_direct! {"Item 4"}, bordered()),
			(rsx_direct! {"Item 5"}, bordered()),
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
	(LayoutStyle::flex_col().row_gap(1), children![
		(
			LayoutStyle::flex_row().column_gap(1),
			children![
				(rsx_direct! {"Header L"}, bordered(), header_style.clone()),
				(rsx_direct! {"Header R"}, bordered(), header_style.clone()),
			],
			bordered()
		),
		(
			LayoutStyle::flex_row().column_gap(1),
			VisualStyle::default()
				.with_background(palettes::tailwind::EMERALD_900),
			children![
				(rsx_direct! {"Sidebar"}, bordered(), sidebar_style),
				(
					LayoutStyle::flex_col().row_gap(1),
					children![
						(rsx_direct! {"Main"}, bordered(), main_style),
						(rsx_direct! {"Footer"}, bordered(), footer_style),
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
	(LayoutStyle::flex_row().column_gap(0), children![
		(rsx_direct! {"A"}, margin()),
		(rsx_direct! {"B"}, margin()),
		(rsx_direct! {"C"}, margin()),
	])
}

fn setup_border_only() -> impl Bundle {
	(LayoutStyle::flex_row().column_gap(0), children![
		(rsx_direct! {"A"}, bordered()),
		(rsx_direct! {"B"}, bordered()),
		(rsx_direct! {"C"}, bordered()),
	])
}

fn setup_padding_only() -> impl Bundle {
	let style = bordered().with_padding(Spacing::all(Length::Rem(1.)));
	(LayoutStyle::flex_row().column_gap(0), children![
		(rsx_direct! {"A"}, style.clone()),
		(rsx_direct! {"B"}, style.clone()),
		(rsx_direct! {"C"}, style.clone()),
	])
}

fn setup_all_spacing() -> impl Bundle {
	let style = BoxStyle::default()
		.with_margin(Spacing::all(Length::Rem(1.)))
		.with_border(Spacing::all(Length::Rem(1.)))
		.with_padding(Spacing::all(Length::Rem(1.)));

	(LayoutStyle::flex_row().column_gap(0), children![
		(rsx_direct! {"A"}, style.clone()),
		(rsx_direct! {"B"}, style.clone()),
		(rsx_direct! {"C"}, style.clone()),
	])
}

// ── Style / Color ─────────────────────────────────────────────────────────────

fn setup_foreground_color() -> impl Bundle {
	(LayoutStyle::flex_row().column_gap(1), children![
		(rsx_direct! { "Red" }, bordered(), VisualStyle {
			foreground: Some(Color::srgb(1., 0., 0.)),
			..default()
		},),
		(rsx_direct! { "Green" }, bordered(), VisualStyle {
			foreground: Some(Color::srgb(0., 0.8, 0.)),
			..default()
		},),
		(rsx_direct! { "Blue" }, bordered(), VisualStyle {
			foreground: Some(Color::srgb(0.2, 0.4, 1.)),
			..default()
		},),
	])
}

fn setup_background_color() -> impl Bundle {
	(LayoutStyle::flex_row().column_gap(1), children![
		(
			rsx_direct! { "A" },
			bordered().with_padding(Spacing::all(Length::Rem(0.5))),
			VisualStyle {
				background: Some(Color::srgb(0.5, 0., 0.5)),
				foreground: Some(Color::WHITE),
				..default()
			},
		),
		(
			rsx_direct! { "B" },
			bordered().with_padding(Spacing::all(Length::Rem(0.5))),
			VisualStyle {
				background: Some(Color::srgb(0., 0.4, 0.6)),
				foreground: Some(Color::WHITE),
				..default()
			},
		),
	])
}

fn setup_border_color() -> impl Bundle {
	// Each node gets per-side border colors: top=red, bottom=blue, left=green, right=yellow
	(LayoutStyle::flex_row().column_gap(1), children![(
		rsx_direct! { "Box" },
		BoxStyle {
			border: Spacing::all(Length::Rem(1.)),
			border_top: Some(Color::srgb(1., 0., 0.)),
			border_bottom: Some(Color::srgb(0., 0.4, 1.)),
			border_left: Some(Color::srgb(0., 0.8, 0.)),
			border_right: Some(Color::srgb(1., 0.8, 0.)),
			..default()
		},
	),])
}

fn setup_text_formatting() -> impl Bundle {
	(LayoutStyle::flex_row().column_gap(1), children![
		(rsx_direct! { "Underline" }, bordered(), VisualStyle {
			decoration_line: DecorationLine::underline(),
			..default()
		},),
		(rsx_direct! { "Strike" }, bordered(), VisualStyle {
			decoration_line: DecorationLine::line_through(),
			..default()
		},),
		(rsx_direct! { "Bold" }, bordered(), VisualStyle {
			font_weight: FontWeight::Bold,
			..default()
		},),
	])
}

fn setup_text_italic() -> impl Bundle {
	(LayoutStyle::flex_row().column_gap(1), children![
		(rsx_direct! { "Italic" }, bordered(), VisualStyle {
			font_style: FontStyle::Italic,
			..default()
		}),
		(rsx_direct! { "Bold+Italic" }, bordered(), VisualStyle {
			font_weight: FontWeight::Bold,
			font_style: FontStyle::Italic,
			..default()
		}),
	])
}

fn setup_text_blink() -> impl Bundle {
	(LayoutStyle::flex_row().column_gap(1), children![
		(rsx_direct! { "Blink" }, bordered(), VisualStyle {
			blink: BlinkStyle::Blink,
			..default()
		}),
		(rsx_direct! { "RapidBlink" }, bordered(), VisualStyle {
			blink: BlinkStyle::RapidBlink,
			..default()
		}),
	])
}

fn setup_text_hidden() -> impl Bundle {
	(LayoutStyle::flex_row().column_gap(1), children![
		(rsx_direct! { "Visible" }, bordered(), VisualStyle {
			visibility: Visibility::Visible,
			..default()
		}),
		(rsx_direct! { "Hidden" }, bordered(), VisualStyle {
			visibility: Visibility::Hidden,
			..default()
		}),
	])
}

fn setup_inline_basic() -> impl Bundle {
	(
		LayoutStyle {
			display: Display::Inline,
			..default()
		},
		children![
			rsx_direct! { "Hello" },
			rsx_direct! { " " },
			rsx_direct! { "World" },
			rsx_direct! { " " },
			rsx_direct! { "Inline!" },
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
			rsx_direct! { "A Very Long Sentence " },
			rsx_direct! { "A Very Long Sentence " },
			rsx_direct! { "A Very Long Sentence " },
			rsx_direct! { "A Very Long Sentence " },
			rsx_direct! { "A Very Long Sentence " },
			rsx_direct! { "A Very Long Sentence " },
		],
	)
}

fn setup_wide_chars() -> impl Bundle {
	// CJK and fullwidth characters occupy 2 terminal columns each
	(LayoutStyle::flex_row().column_gap(1), children![
		(rsx_direct! { "中文" }, bordered(), VisualStyle {
			foreground: Some(Color::srgb(0.9, 0.5, 0.1)),
			..default()
		}),
		(rsx_direct! { "日本語" }, bordered(), VisualStyle {
			foreground: Some(Color::srgb(0.1, 0.7, 0.9)),
			..default()
		}),
		(rsx_direct! { "ＡＢＣ" }, bordered(), VisualStyle {
			foreground: Some(Color::srgb(0.5, 0.9, 0.4)),
			..default()
		}),
	])
}

fn setup_text_align() -> impl Bundle {
	let item_styles = (
		BoxStyle::default()
			.with_padding(Spacing::all(Length::Rem(2.)))
			.with_border(Spacing::all(Length::Rem(1.))),
		LayoutStyle::default().with_flex_grow(1),
	);

	(LayoutStyle::flex_row().column_gap(1), children![
		(rsx_direct! { "Left" }, item_styles.clone(), VisualStyle {
			text_align: TextAlign::Left,
			..default()
		},),
		(rsx_direct! { "Center" }, item_styles.clone(), VisualStyle {
			text_align: TextAlign::Center,
			..default()
		},),
		(rsx_direct! { "Right" }, item_styles, VisualStyle {
			text_align: TextAlign::Right,
			..default()
		},),
	])
}
