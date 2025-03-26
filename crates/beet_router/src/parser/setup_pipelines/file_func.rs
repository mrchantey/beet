use anyhow::Result;
use beet_rsx::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;


pub struct FileFunc<T> {
	/// The path relative to its root, the [`FileGroup::src`] it was collected from.
	/// This is useful for generating route paths.
	pub local_path: PathBuf,
	/// The function name
	pub name: String,
	pub func: T,
}

impl<T> FileFunc<T> {
	pub fn new<M>(
		name: impl Into<String>,
		local_path: impl AsRef<Path>,
		func: impl IntoFileFunc<T, M>,
	) -> Self {
		Self {
			name: name.into(),
			local_path: local_path.as_ref().into(),
			func: func.into_file_func(),
		}
	}
}

/// A mechanic that allows great flexibility in the kinds of
/// functions that can be collected.
pub trait IntoFileFunc<T, M>: 'static {
	fn into_file_func(self) -> T;
}

pub type DefaultFileFunc =
	Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<RsxRoot>>>>>;


impl<F> IntoFileFunc<DefaultFileFunc, ()> for F
where
	F: 'static + Clone + Fn() -> RsxRoot,
{
	fn into_file_func(self) -> DefaultFileFunc {
		Box::new(move || {
			// why clone?
			let func = self.clone();
			Box::pin(async move { Ok(func()) })
		})
	}
}


pub struct AsyncFileFuncMarker;

impl<F> IntoFileFunc<DefaultFileFunc, AsyncFileFuncMarker> for F
where
	F: 'static + Clone + AsyncFn() -> RsxRoot,
{
	fn into_file_func(self) -> DefaultFileFunc {
		Box::new(move || {
			let func = self.clone();
			Box::pin(async move { Ok(func().await) })
		})
	}
}
