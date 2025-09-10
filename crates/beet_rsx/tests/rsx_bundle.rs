// probs should be a test but so nice for cargo expand
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_rsx::prelude::*;
use bevy::prelude::*;

fn main() {
	App::new()
		.world_mut()
		.spawn(MyBundle::default().into_bundle());
}

#[derive(Default, Buildable, AttributeBlock)]
struct MyBundle {
	/// the class that will be set
	class: String,
	/// this is what identifies it
	id: Option<String>,
	disabled: Option<bool>,
	// onclick: Option<Box<dyn EventHandler<MouseEvent>>>,
}
