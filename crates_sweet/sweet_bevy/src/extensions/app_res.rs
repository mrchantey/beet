use bevy::prelude::*;
use extend::ext;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Deref, DerefMut)]
pub struct AppRes(pub Rc<RefCell<App>>);

impl AppRes {
	pub fn new() -> Rc<RefCell<App>> { Self::init(App::new()) }


	pub fn build(func: impl FnOnce(&mut App)) -> Rc<RefCell<App>> {
		let mut app = App::new();
		func(&mut app);
		Self::init(app)
	}

	pub fn init(app: App) -> Rc<RefCell<App>> {
		let app = Rc::new(RefCell::new(app));
		let app2 = app.clone();
		app.borrow_mut().insert_non_send_resource(AppRes(app2));
		app
	}
}

#[ext]
pub impl Rc<RefCell<App>> {
	#[cfg(target_arch = "wasm32")]
	fn run_on_animation_frame(self) -> sweet_web::AnimationFrame {
		sweet_web::AnimationFrame::new(move || {
			self.borrow_mut().update();
		})
	}

	#[cfg(target_arch = "wasm32")]
	fn run_forever(self) -> impl std::future::Future<Output = ()> {
		async {
			let _frame = self.run_on_animation_frame();
			sweet_web::loop_forever().await;
		}
	}

	#[cfg(target_arch = "wasm32")]
	fn run_while_mounted(self) {
		todo!("broken in 12.1?");
		// let mut app = self.borrow_mut();
		// app.finish();
		// app.cleanup();
		// drop(app);
	}
}
