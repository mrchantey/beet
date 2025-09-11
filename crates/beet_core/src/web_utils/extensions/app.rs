use crate::prelude::*;
use bevy::prelude::*;
#[extend::ext(name=BeetDomAppExt)]
pub impl App {
	// TODO this is a runner not extension
	fn run_on_animation_frame(mut self) -> AnimationFrame {
		AnimationFrame::new(move || {
			self.update();
		})
	}
}
