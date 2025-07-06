use extend::ext;

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
				crate::log!("{err}");
				std::process::exit(1);
			}
		}
	}
}


// #[ext(name =ResultExtEDebug)]
// pub impl<T, E: std::fmt::Debug> Result<T, E> {
// 	/// Map a `Result<T,E:Debug>` to an [`anyhow::Result`].
// 	fn anyhow(self) -> anyhow::Result<T> {
// 		match self {
// 			Ok(v) => Ok(v),
// 			Err(e) => Err(anyhow::anyhow!("{:?}", e)),
// 		}
// 	}
// }
