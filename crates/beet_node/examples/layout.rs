// use beet_core::prelude::*;
use beet_node::prelude::*;


fn main() {
	let btn = |label: &str| {
		FlexChild::new(Button {
			label: label.into(),
		})
	};

	let toolbar = FlexBox::row()
		.wrap(FlexWrap::Wrap)
		.align_items(AlignItems::Start)
		.child(btn("Save")) // 8 wide
		.child(btn("Cancel")) // 10 wide
		.child(btn("Delete")) // 10 wide
		.child(btn("Preview")) // 11 wide
		.child(
			FlexChild::new(
				// spacer, grows to fill remaining row space
				TextWidget::new("foobar bazz"),
			)
			.grow(1),
		);

	let mut cells = Vec::new();
	toolbar.layout(
		Rect {
			x: 0,
			y: 0,
			w: 24,
			h: 10,
		},
		&mut cells,
	);
	println!("{}", render(&cells, 24, 10));
}
