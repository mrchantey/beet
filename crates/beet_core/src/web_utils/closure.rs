//! Closure construction helpers for `wasm_bindgen` closures.
//!
//! Provides extension methods that allow creating [`Closure`] instances
//! without explicit type annotations.

use extend::ext;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::closure::IntoWasmClosure;
use wasm_bindgen::closure::WasmClosure;

/// Extension trait for creating single-argument wasm closures.
#[ext]
pub impl<T1, T2> Closure<dyn FnMut(T1) -> T2> {
	/// Creates a new closure without explicit type annotations.
	///
	/// Equivalent to [`Closure::new`] but with better type inference.
	fn from_func<F>(func: F) -> Self
	where
		dyn FnMut(T1) -> T2: WasmClosure,
		F: IntoWasmClosure<dyn FnMut(T1) -> T2> + 'static,
	{
		Closure::new(func)
	}
}

/// Extension trait for creating zero-argument wasm closures.
#[ext]
pub impl<T2> Closure<dyn FnMut() -> T2> {
	/// Creates a new zero-argument closure without explicit type annotations.
	///
	/// Equivalent to [`Closure::new`] but with better type inference.
	fn from_func_no_args<F>(func: F) -> Self
	where
		dyn FnMut() -> T2: WasmClosure,
		F: IntoWasmClosure<dyn FnMut() -> T2> + 'static,
	{
		Closure::new(func)
	}
}
