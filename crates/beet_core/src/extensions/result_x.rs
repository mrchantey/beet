use extend::ext;

#[cfg(target_arch = "wasm32")]
use crate::web_utils::js_runtime;

#[ext]
pub impl<T, E> Result<T, E> {
	fn ok_or(self, func: impl FnOnce(E)) -> Option<T> {
		match self {
			Ok(value) => Some(value),
			Err(err) => {
				func(err);
				None
			}
		}
	}
}



#[ext(name=ResultExtDisplay)]
pub impl<T, E: std::fmt::Display> Result<T, E> {
	/// Print a more nicely formatted error message
	/// than `.unwrap()`
	fn unwrap_or_exit(self) -> T {
		match self {
			Ok(value) => value,
			Err(err) => {
				eprintln!("{err}");
				#[cfg(not(target_arch = "wasm32"))]
				std::process::exit(1);
				#[cfg(target_arch = "wasm32")]
				{
					js_runtime::exit(1);
					#[allow(clippy::empty_loop)]
					loop {}
				}
			}
		}
	}
}
