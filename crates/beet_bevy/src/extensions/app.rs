use bevy::prelude::*;
use extend::ext;

#[ext]
#[allow(async_fn_in_trait)]
pub impl App {
	#[cfg(target_arch = "wasm32")]
	fn run_on_animation_frame(mut self) -> beet_web::AnimationFrame {
		beet_web::AnimationFrame::new(move || {
			self.update();
		})
	}

	/// Convenience method for running with an async runner,
	/// this will [`std::mem::take`] the app and pass it to the runner.
	async fn run_async(
		&mut self,
		runner: impl AsyncFnOnce(App) -> AppExit + 'static,
	) -> AppExit {
		let app = std::mem::take(self);
		runner(app).await
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;
	#[test]
	pub fn works() {
		let app = AppRes::new();
		let app = app.borrow_mut();
		expect(app.world().contains_non_send::<AppRes>()).to_be_true();
	}
}
