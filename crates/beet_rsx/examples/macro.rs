use beet_rsx::as_beet::*;
use bevy::prelude::*;

fn main() {
	App::new()
		.world_mut()
		.spawn(rsx! {
		<div
			onclick={||println!("clicked")}>
			"hello world!"
		</div>
		})
		.flush_trigger(OnClick);
}
