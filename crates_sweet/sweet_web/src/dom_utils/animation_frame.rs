use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::window;

pub struct AnimationFrame(pub Rc<RefCell<i32>>);


impl AnimationFrame {
	pub fn new<F>(mut on_frame: F) -> Self
	where
		F: FnMut() + 'static,
	{
		let f = Rc::new(RefCell::new(None));
		let handle = Rc::new(RefCell::new(0));
		let handle2 = handle.clone();
		let f2 = f.clone();
		*f2.borrow_mut() = Some(Closure::new(move || {
			on_frame();
			*handle2.borrow_mut() =
				request_animation_frame(f.borrow().as_ref().unwrap());
		}));
		*handle.borrow_mut() =
			request_animation_frame(f2.borrow().as_ref().unwrap());
		Self(handle)
	}
	pub fn forget(self) { *self.0.borrow_mut() = i32::MAX; }
}

impl Drop for AnimationFrame {
	fn drop(&mut self) {
		window()
			.unwrap()
			.cancel_animation_frame(*self.0.borrow())
			.expect("failed to request animation frame");
	}
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) -> i32 {
	window()
		.unwrap()
		.request_animation_frame(f.as_ref().unchecked_ref())
		.expect("failed to request animation frame")
}
