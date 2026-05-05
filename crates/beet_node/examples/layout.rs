use beet_core::prelude::*;
use beet_node::prelude::style::AlignItems;
use beet_node::prelude::style::FlexWrap;
use beet_node::prelude::style::JustifyContent;
use beet_node::prelude::style::*;
use beet_node::prelude::*;
use beet_node::*;

fn main() {
	println!("=== Beet Layout Engine Demo ===\n");

	render("JustifyContent::Start", setup_justify_start);
	render("JustifyContent::Center", setup_justify_center);
	render("JustifyContent::End", setup_justify_end);
	render("JustifyContent::SpaceBetween", setup_justify_space_between);
	render("JustifyContent::SpaceAround", setup_justify_space_around);
	render("JustifyContent::SpaceEvenly", setup_justify_space_evenly);

	render("AlignItems::Start", setup_align_start);
	render("AlignItems::Center", setup_align_center);
	render("AlignItems::End", setup_align_end);
	render("AlignItems::Stretch", setup_align_stretch);

	render("Row and Column Gaps", setup_gaps);
	render("Flex Grow", setup_flex_grow);
	render("No Wrap", setup_no_wrap);
	render("Wrap", setup_wrap);
	render("Nested Rows and Columns", setup_nested);

	render("Margin Only", setup_margin_only);
	render("Border Only", setup_border_only);
	render("Padding Only", setup_padding_only);
	render("Margin + Border + Padding", setup_all_spacing);

	// Style demos (ANSI color output)
	render("Foreground Color", setup_foreground_color);
	render("Background Color", setup_background_color);
	render("Border Color (per-side)", setup_border_color);
	render("Text Underline", setup_text_underline);
	render("Text Align + Style", setup_text_align_style);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn render<B: Bundle>(name: &str, setup: fn() -> B) {
	let out = RenderCharcell::default()
		.render_oneshot(setup())
		.unwrap()
		.render()
		.trim_lines();
	println!("\n{name}: \n{out}");
}

fn bordered() -> LayoutStyle {
	LayoutStyle::default().with_border(Spacing::all(Length::Rem(1.)))
}

fn margin() -> LayoutStyle {
	LayoutStyle::default().with_margin(Spacing::all(Length::Rem(1.)))
}


// ── JustifyContent ────────────────────────────────────────────────────────────

fn setup_justify_start() -> impl Bundle {
	(
		FlexBox::row()
			.justify_content(JustifyContent::Start)
			.column_gap(1),
		children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
			(rsx! {"C"}, bordered()),
		],
	)
}

fn setup_justify_center() -> impl Bundle {
	(
		FlexBox::row()
			.justify_content(JustifyContent::Center)
			.column_gap(1),
		children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
			(rsx! {"C"}, bordered()),
		],
	)
}

fn setup_justify_end() -> impl Bundle {
	(
		FlexBox::row()
			.justify_content(JustifyContent::End)
			.column_gap(1),
		children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
			(rsx! {"C"}, bordered()),
		],
	)
}

fn setup_justify_space_between() -> impl Bundle {
	(
		FlexBox::row()
			.justify_content(JustifyContent::SpaceBetween)
			.column_gap(1),
		children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
			(rsx! {"C"}, bordered()),
		],
	)
}

fn setup_justify_space_around() -> impl Bundle {
	(
		FlexBox::row()
			.justify_content(JustifyContent::SpaceAround)
			.column_gap(1),
		children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
			(rsx! {"C"}, bordered()),
		],
	)
}

fn setup_justify_space_evenly() -> impl Bundle {
	(
		FlexBox::row()
			.justify_content(JustifyContent::SpaceEvenly)
			.column_gap(1),
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
		FlexBox::row().align_items(AlignItems::Start).column_gap(1),
		children![
			(
				FlexBox::col(),
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
		FlexBox::row().align_items(AlignItems::Center).column_gap(1),
		children![
			(
				FlexBox::col(),
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
		FlexBox::row().align_items(AlignItems::End).column_gap(1),
		children![
			(
				FlexBox::col(),
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
		FlexBox::row()
			.align_items(AlignItems::Stretch)
			.column_gap(1),
		children![
			(
				FlexBox::col(),
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
		FlexBox::row().wrap(FlexWrap::Wrap).row_gap(1).column_gap(2),
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
	(FlexBox::row().column_gap(1), children![
		(rsx! {"Fixed"}, bordered()),
		(rsx! {"Grow 1"}, bordered().with_flex_grow(1)),
		(rsx! {"Grow 2"}, bordered().with_flex_grow(2)),
		(rsx! {"Fixed"}, bordered()),
	])
}

// ── Wrapping ──────────────────────────────────────────────────────────────────

fn setup_no_wrap() -> impl Bundle {
	(FlexBox::row().column_gap(1), children![
		(rsx! {"Item 1"}, bordered()),
		(rsx! {"Item 2"}, bordered()),
		(rsx! {"Item 3"}, bordered()),
		(rsx! {"Item 4"}, bordered()),
		(rsx! {"Item 5"}, bordered()),
	])
}

fn setup_wrap() -> impl Bundle {
	(
		FlexBox::row().wrap(FlexWrap::Wrap).column_gap(1).row_gap(1),
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
	(FlexBox::col().row_gap(1), children![
		(
			FlexBox::row().column_gap(1),
			children![
				(rsx! {"Header L"}, bordered()),
				(rsx! {"Header R"}, bordered()),
			],
			bordered()
		),
		(
			FlexBox::row().column_gap(1),
			children![
				(rsx! {"Sidebar"}, bordered()),
				(
					FlexBox::col().row_gap(1),
					children![
						(rsx! {"Main"}, bordered()),
						(rsx! {"Footer"}, bordered()),
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
	(FlexBox::row().column_gap(0), children![
		(rsx! {"A"}, margin()),
		(rsx! {"B"}, margin()),
		(rsx! {"C"}, margin()),
	])
}

fn setup_border_only() -> impl Bundle {
	(FlexBox::row().column_gap(0), children![
		(rsx! {"A"}, bordered()),
		(rsx! {"B"}, bordered()),
		(rsx! {"C"}, bordered()),
	])
}

fn setup_padding_only() -> impl Bundle {
	let style = bordered().with_padding(Spacing::all(Length::Rem(1.)));
	(FlexBox::row().column_gap(0), children![
		(rsx! {"A"}, style.clone()),
		(rsx! {"B"}, style.clone()),
		(rsx! {"C"}, style.clone()),
	])
}

fn setup_all_spacing() -> impl Bundle {
	let style = LayoutStyle::default()
		.with_margin(Spacing::all(Length::Rem(1.)))
		.with_border(Spacing::all(Length::Rem(1.)))
		.with_padding(Spacing::all(Length::Rem(1.)));


	(FlexBox::row().column_gap(0), children![
		(rsx! {"A"}, style.clone()),
		(rsx! {"B"}, style.clone()),
		(rsx! {"C"}, style.clone()),
	])
}

// ── Style / Color ────────────────────────────────────────────────────────────────────────────

fn setup_foreground_color() -> impl Bundle {
	(FlexBox::row().column_gap(1), children![
		(rsx! { "Red" }, bordered(), VisualStyle {
			foreground: Some(Color::srgb(1., 0., 0.)),
			..VisualStyle::default()
		},),
		(rsx! { "Green" }, bordered(), VisualStyle {
			foreground: Some(Color::srgb(0., 0.8, 0.)),
			..VisualStyle::default()
		},),
		(rsx! { "Blue" }, bordered(), VisualStyle {
			foreground: Some(Color::srgb(0.2, 0.4, 1.)),
			..VisualStyle::default()
		},),
	])
}

fn setup_background_color() -> impl Bundle {
	(FlexBox::row().column_gap(1), children![
		(
			rsx! { "A" },
			bordered().with_padding(Spacing::all(Length::Rem(0.5))),
			VisualStyle {
				background: Some(Color::srgb(0.5, 0., 0.5)),
				foreground: Some(Color::WHITE),
				..VisualStyle::default()
			},
		),
		(
			rsx! { "B" },
			bordered().with_padding(Spacing::all(Length::Rem(0.5))),
			VisualStyle {
				background: Some(Color::srgb(0., 0.4, 0.6)),
				foreground: Some(Color::WHITE),
				..VisualStyle::default()
			},
		),
	])
}

fn setup_border_color() -> impl Bundle {
	// Each node gets per-side border colors: top=red, bottom=blue, left=green, right=yellow
	(FlexBox::row().column_gap(1), children![(
		rsx! { "Box" },
		bordered(),
		VisualStyle {
			border_top: Some(Color::srgb(1., 0., 0.)),
			border_bottom: Some(Color::srgb(0., 0.4, 1.)),
			border_left: Some(Color::srgb(0., 0.8, 0.)),
			border_right: Some(Color::srgb(1., 0.8, 0.)),
			..VisualStyle::default()
		},
	),])
}

fn setup_text_underline() -> impl Bundle {
	(FlexBox::row().column_gap(1), children![
		(rsx! { "Underline" }, bordered(), VisualStyle {
			text_style: TextStyle::UNDERLINE,
			..VisualStyle::default()
		},),
		(rsx! { "Strike" }, bordered(), VisualStyle {
			text_style: TextStyle::LINE_THROUGH,
			..VisualStyle::default()
		},),
	])
}

fn setup_text_align_style() -> impl Bundle {
	(FlexBox::row().column_gap(1), children![
		(
			rsx! { "Left" },
			LayoutStyle::default()
				.with_border(Spacing::all(Length::Rem(1.)))
				.with_text_align(TextAlign::Left),
			VisualStyle {
				foreground: Some(Color::srgb(0.8, 0.4, 0.)),
				..VisualStyle::default()
			},
		),
		(
			rsx! { "Center" },
			LayoutStyle::default()
				.with_border(Spacing::all(Length::Rem(1.)))
				.with_text_align(TextAlign::Center),
			VisualStyle {
				foreground: Some(Color::srgb(0., 0.8, 0.8)),
				..VisualStyle::default()
			},
		),
	])
}
