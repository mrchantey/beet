use beet_core::prelude::*;
use beet_ui::prelude::*;


fn main() {
	App::new()
		.add_plugins(TokenPlugin)
		.add_systems(Startup, setup)
		.add_systems(Update, (update1, update2).chain())
		.run();
}

fn count_def() -> TokenDefinition<i32> { TokenDefinition::inline(7) }

fn setup(world: &mut World) {
	let count = count_def();
	world
		.spawn(count.into_bundle())
		.get::<Value>()
		.unwrap()
		.xpect_eq(Value::Int(7));
}


fn update1(mut commands: Commands, query: Query<Entity>) -> Result {
	let entity = query.single()?;
	let count = count_def();
	commands
		.entity(entity)
		.queue(count.update(|prev| *prev += 1));
	Ok(())
}
// new value after command flushed
fn update2(query: Query<&Value>) -> Result {
	query.single()?.xpect_eq(Value::Int(8));
	println!("success");
	Ok(())
}
