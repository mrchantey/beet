use beet_node::prelude::*;
use bevy::math::URect;


fn main() {
	// demonstrate all layout features with a reasonable terminal size
	let width = 80;
	let _height = 50;

	println!("=== Beet Layout Engine Demo ===\n");

	demo_justify_content(width);
	demo_align_items(width);
	demo_gaps(width);
	demo_flex_grow(width);
	demo_wrapping(width);
	demo_nested(width);
}

fn demo_justify_content(width: u32) {
	println!("--- JustifyContent ---");

	for (name, justify) in [
		("Start", JustifyContent::Start),
		("Center", JustifyContent::Center),
		("End", JustifyContent::End),
		("SpaceBetween", JustifyContent::SpaceBetween),
		("SpaceAround", JustifyContent::SpaceAround),
		("SpaceEvenly", JustifyContent::SpaceEvenly),
	] {
		let layout = FlexBox::row()
			.justify_content(justify)
			.column_gap(1)
			.child(Bordered::new(TextWidget::new("A")))
			.child(Bordered::new(TextWidget::new("B")))
			.child(Bordered::new(TextWidget::new("C")));

		render_demo(name, layout, width, 5);
	}
}

fn demo_align_items(width: u32) {
	println!("--- AlignItems ---");

	for (name, align) in [
		("Start", AlignItems::Start),
		("Center", AlignItems::Center),
		("End", AlignItems::End),
		("Stretch", AlignItems::Stretch),
	] {
		let layout = FlexBox::row()
			.align_items(align)
			.column_gap(1)
			.child(Bordered::new(
				FlexBox::col()
					.child(TextWidget::new("Very"))
					.child(TextWidget::new("Tall"))
					.child(TextWidget::new("Item")),
			))
			.child(Bordered::new(TextWidget::new("Short")))
			.child(Bordered::new(TextWidget::new("B")));

		render_demo(name, layout, width, 6);
	}
}

fn demo_gaps(_width: u32) {
	println!("--- Row and Column Gaps ---");

	let layout = FlexBox::row()
		.wrap(FlexWrap::Wrap)
		.row_gap(1)
		.column_gap(2)
		.child(Bordered::new(TextWidget::new("1")))
		.child(Bordered::new(TextWidget::new("2")))
		.child(Bordered::new(TextWidget::new("3")))
		.child(Bordered::new(TextWidget::new("4")))
		.child(Bordered::new(TextWidget::new("5")))
		.child(Bordered::new(TextWidget::new("6")));

	render_demo("Gaps with wrapping", layout, 20, 12);
}

fn demo_flex_grow(width: u32) {
	println!("--- Flex Grow ---");

	let layout = FlexBox::row()
		.column_gap(1)
		.child(Bordered::new(TextWidget::new("Fixed")))
		.child(Bordered::new(TextWidget::new("Grow 1")).flex_grow(1))
		.child(Bordered::new(TextWidget::new("Grow 2")).flex_grow(2))
		.child(Bordered::new(TextWidget::new("Fixed")));

	render_demo("Different grow values", layout, width, 5);
}

fn demo_wrapping(_width: u32) {
	println!("--- Wrapping ---");

	let layout = FlexBox::row()
		.wrap(FlexWrap::Wrap)
		.column_gap(1)
		.row_gap(1)
		.child(Bordered::new(TextWidget::new("Button 1")))
		.child(Bordered::new(TextWidget::new("Button 2")))
		.child(Bordered::new(TextWidget::new("Button 3")))
		.child(Bordered::new(TextWidget::new("Button 4")))
		.child(Bordered::new(TextWidget::new("Button 5")));

	render_demo("Wrapping at container edge", layout, 40, 12);
}

fn demo_nested(width: u32) {
	println!("--- Nested Layout ---");

	let header = Bordered::new(
		TextWidget::new("Application Header").align(TextAlign::Center),
	);

	let content = FlexBox::row()
		.column_gap(1)
		.child(
			Bordered::new(
				FlexBox::col()
					.child(TextWidget::new("Sidebar"))
					.child(TextWidget::new("Item 1"))
					.child(TextWidget::new("Item 2")),
			)
			.flex_grow(1),
		)
		.child(
			Bordered::new(TextWidget::new(
				"Main content area grows to fill space",
			))
			.flex_grow(3),
		);

	let footer = FlexBox::row()
		.column_gap(1)
		.child(Bordered::new(TextWidget::new("Save")))
		.child(TextWidget::new("Status: OK").flex_grow(1))
		.child(Bordered::new(TextWidget::new("Help")));

	let app = FlexBox::col()
		.row_gap(1)
		.child(header)
		.child(content)
		.child(footer);

	render_demo("Complete app layout", app, width, 20);
}

fn render_demo(title: &str, widget: impl Widget, width: u32, height: u32) {
	let rect = URect::new(0, 0, width, height);
	let mut buffer = Buffer::new(rect);
	widget.layout(&mut buffer, rect);

	println!("{}: ", title);
	println!("{}", buffer.render_plain());
	println!();
}
