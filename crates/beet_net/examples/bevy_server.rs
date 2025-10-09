use beet_core::prelude::*;
use beet_net::prelude::*;

#[rustfmt::skip]
fn main() {
	App::new()
		.add_plugins((DefaultPlugins, ServerPlugin))
		.run();
}
