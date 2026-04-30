use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((MinimalPlugins, TuiPlugin2))
		.add_systems(Startup, setup)
		.run();
}

fn token() {
	todo!();
}
fn increment(_token: ()) {
	todo!();
}
fn setup(mut commands: Commands) {
	let value_token = token();
	let increment_token = increment(value_token);

	commands.spawn(rsx! {
		<div>
			<span> Count: {value_token}</span>
			<button onclick={increment_token}>Increment</button>
		</div>
	});
}
