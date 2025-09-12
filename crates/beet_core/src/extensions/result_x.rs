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
				eprintln!("{err}");
				std::process::exit(1);
			}
		}
	}
}
