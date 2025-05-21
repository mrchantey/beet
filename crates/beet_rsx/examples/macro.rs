use beet_rsx::as_beet::*;
use bevy::prelude::*;

fn main() {
	App::new()
		.world_mut()
		.spawn(rsx! {<button onclick={||println!("clicked")}/>})
		.trigger(OnClick);
}
