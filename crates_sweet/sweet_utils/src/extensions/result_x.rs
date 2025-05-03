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
// #[ext]
// pub impl<T, E: Display> Result<T, E> {
// Consume the error, log it and return None, otherwise return the value.
// fn log_err(self) -> Option<T> {
// 	match self {
// 		Ok(value) => Some(value),
// 		Err(err) => {
// 			log::error!("{err}");
// 			None
// 		}
// 	}
// }
// }


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
