use crate::prelude::NonSendPlugin;
use bevy::app::PluginsState;
use bevy::prelude::*;
use extend::ext;

#[ext]
#[allow(async_fn_in_trait)]
pub impl App {
	#[cfg(all(target_arch = "wasm32", feature = "web"))]
	fn run_on_animation_frame(mut self) -> crate::web::AnimationFrame {
		crate::web::AnimationFrame::new(move || {
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

	fn run_once(&mut self) -> AppExit {
		self.init();
		self.update();
		self.should_exit().unwrap_or(AppExit::Success)
	}

	/// Call this on custom runners before update
	/// to ensure that the app is fully initialized.
	// from bevy_app https://github.com/mrchantey/bevy/blob/a1f4e56610c090b44f8b4a8f3eb56aeda5eb9669/crates/bevy_app/src/app.rs#L1392
	fn init(&mut self) -> &mut Self {
		while self.plugins_state() == PluginsState::Adding {
			#[cfg(not(target_arch = "wasm32"))]
			bevy::tasks::tick_global_task_pools_on_main_thread();
		}
		self.finish();
		self.cleanup();
		self
	}

	fn add_non_send_plugin(&mut self, plugin: impl NonSendPlugin) -> &mut Self {
		plugin.build(self);
		self
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
