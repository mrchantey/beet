use extend::ext;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::closure::IntoWasmClosure;
use wasm_bindgen::closure::WasmClosure;
// use Sized;

#[ext]
pub impl<T1, T2> Closure<dyn FnMut(T1) -> T2> {
	/// Equivelant of `new` but without need for explicit type annotations
	fn from_func<F>(func: F) -> Self
	where
		dyn FnMut(T1) -> T2: WasmClosure,
		F: IntoWasmClosure<dyn FnMut(T1) -> T2> + 'static,
	{
		Closure::new(func)
	}
}
#[ext]
pub impl<T2> Closure<dyn FnMut() -> T2> {
	/// Equivelant of `new` but without need for explicit type annotations
	fn from_func_no_args<F>(func: F) -> Self
	where
		dyn FnMut() -> T2: WasmClosure,
		F: IntoWasmClosure<dyn FnMut() -> T2> + 'static,
	{
		Closure::new(func)
	}
}
// where
// 	T: WasmClosure,
// {
// 	fn from_func<F>(func: F) -> Self
// 	where
// 		F: IntoWasmClosure<T> + 'static,
// 	{
// 		Self::new(func)
// 	}
// }
