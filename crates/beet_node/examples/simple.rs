use beet_core::prelude::*;
use beet_node::prelude::*;
// use beet_node::*;
// use bevy::math::URect;


fn main() {
	App::new()
		// .add_plugins(TuiPlugin2)
		.add_systems(Startup, setup)
		.add_systems(Update, update)
		.run();
}



fn setup(mut commands: Commands) {
	// commands.spawn(rsx! {<div>hello world</div>});
	let widget = Bordered::new(TextWidget::new("foobar"));
	commands.spawn(EntityWidget::new(widget));
}
fn update(elements: Query<Entity>, query: WidgetQuery) -> Result {
	let buffer = query.render(elements.single()?)?;
	println!("{}", buffer.render_plain());
	println!();
	todo!("use crates/beet_node/src/render/tui2/render_context.rs");
}


// struct TuiVisitor<'a> {
// 	buffer: &'a mut Buffer,
// }

// impl<'a> NodeVisitor for TuiVisitor<'a> {
// 	fn visit_element(&mut self, _cx: &VisitContext, _view: ElementView) {
// 		todo!("render this")
// 	}
// 	fn visit_value(&mut self, _cx: &VisitContext, _value: &Value) {}
// }
