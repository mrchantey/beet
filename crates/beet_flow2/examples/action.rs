use beet_core::prelude::*;
use beet_flow2::prelude::*;


fn log_on_run(name: impl Into<String>) -> impl Bundle {
	let name = name.into();
	EntityObserver::new(move |_: On<Run>| {
		println!("greetings {}", name);
	})
}

fn main() {
	App::new()
		.world_mut()
		.spawn(log_on_run("bob"))
		.auto_trigger(RUN);
}
