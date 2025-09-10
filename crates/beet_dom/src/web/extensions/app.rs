use bevy::prelude::*;

#[extend::ext(name=BeetDomAppExt)]
pub impl App {
	#[cfg(target_arch = "wasm32")]
	fn run_on_animation_frame(mut self) -> crate::web::AnimationFrame {
		crate::web::AnimationFrame::new(move || {
			self.update();
		})
	}
}
