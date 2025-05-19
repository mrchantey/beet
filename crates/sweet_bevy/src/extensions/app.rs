use bevy::prelude::*;
use extend::ext;

#[ext]
pub impl App {
	#[cfg(target_arch = "wasm32")]
	#[must_use]
	fn run_on_animation_frame(mut self) -> sweet_web::AnimationFrame {
		sweet_web::AnimationFrame::new(move || {
			self.update();
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet_test::prelude::*;
	#[test]
	pub fn works() {
		let app = AppRes::new();
		let app = app.borrow_mut();
		expect(app.world().contains_non_send::<AppRes>()).to_be_true();
	}
}
