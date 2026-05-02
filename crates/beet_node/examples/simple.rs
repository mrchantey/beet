use beet_core::prelude::*;
use beet_node::prelude::style::*;
use beet_node::prelude::*;
use beet_node::*;
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
	// let widget = Bordered::new(TextWidget::new("foobar"));
	// commands.spawn(EntityWidget::new(widget));
	// commands.spawn((rsx! {<div>"hello world!"</div>}, VisualStyle::default()));
	commands.spawn((
		rsx! {"hello world!"},
		VisualStyle::default(),
		LayoutStyle::default()
			.with_margin(Spacing::all(Length::Rem(3.)))
			.with_padding(Spacing::all(Length::Rem(3.))),
	));
}
fn update(
	root: Query<Entity, (Without<ChildOf>, Without<AttributeOf>)>,
	query: StyledNodeQuery,
) -> Result {
	let entity = root.single()?;
	let buffer = TuiRenderContext::render_half(&query, entity)?;
	println!("{}", buffer.render_plain());
	Ok(())
}
