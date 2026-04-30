// use beet_core::prelude::*;
use beet_node::prelude::*;


fn main() {
	let btn = |label: &str| Button {
		label: label.into(),
		layout_style: Default::default(),
	};

	let toolbar = FlexBox::row()
		.wrap(FlexWrap::Wrap)
		.align_items(AlignItems::Start)
		.child(btn("Save")) // 8 wide
		.child(btn("Cancel")) // 10 wide
		.child(btn("Delete")) // 10 wide
		.child(btn("Preview")) // 11 wide
		.child(TextWidget {
			content: "foobar bazz".into(),
			layout_style: LayoutStyle { flex_grow: Some(1) },
			align: TextAlign::Left,
		});

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
