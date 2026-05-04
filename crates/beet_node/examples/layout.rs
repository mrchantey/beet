use beet_core::prelude::*;
use beet_node::prelude::style::AlignItems;
use beet_node::prelude::style::FlexWrap;
use beet_node::prelude::style::JustifyContent;
use beet_node::prelude::style::*;
use beet_node::prelude::*;
use beet_node::*;

fn main() {
	println!("=== Beet Layout Engine Demo ===\n");

	run_demo("JustifyContent::Start", setup_justify_start);
	run_demo("JustifyContent::Center", setup_justify_center);
	run_demo("JustifyContent::End", setup_justify_end);
	run_demo("JustifyContent::SpaceBetween", setup_justify_space_between);
	run_demo("JustifyContent::SpaceAround", setup_justify_space_around);
	run_demo("JustifyContent::SpaceEvenly", setup_justify_space_evenly);

	run_demo("AlignItems::Start", setup_align_start);
	run_demo("AlignItems::Center", setup_align_center);
	run_demo("AlignItems::End", setup_align_end);
	run_demo("AlignItems::Stretch", setup_align_stretch);

	run_demo("Row and Column Gaps", setup_gaps);
	run_demo("Flex Grow", setup_flex_grow);
	run_demo("No Wrap", setup_no_wrap);
	run_demo("Wrap", setup_wrap);
	run_demo("Nested Rows and Columns", setup_nested);

	run_demo("Margin Only", setup_margin_only);
	run_demo("Border Only", setup_border_only);
	run_demo("Padding Only", setup_padding_only);
	run_demo("Margin + Border + Padding", setup_all_spacing);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn run_demo<B: Bundle>(name: &str, setup: fn() -> B) {
	let out = World::new()
		.spawn(setup())
		.with_state::<StyledNodeQuery, _>(|entity, query| {
			let buffer = TuiRenderContext::render_full(&query, entity).unwrap();
			buffer.render_plain_trim()
		});
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
