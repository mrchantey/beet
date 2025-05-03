use wasm_bindgen::prelude::*;

pub fn forget<T>(val: T) {
	let closure = Closure::<dyn FnMut()>::new(move || {
		let _ = val;
	});
	closure.forget();
}
pub fn forget_func<F>(f: F)
where
	F: FnMut() + 'static,
{
	let closure = Closure::<dyn FnMut()>::new(f);
	closure.forget();
}
