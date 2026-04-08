use crate::prelude::*;
use extend::ext;



// #[ext]
// pub impl SmolStr {
// 	/// Appends a string slice to the end of this `SmolStr`, returning a new `SmolStr`.
// 	// fn push(self, s: &str) -> Self { SmolStr::new(format!("{}{}", self, s)) }
// }
/// Extension methods for [`Vec<SmolStr>`] providing additional string manipulation utilities.
#[ext]
pub impl Vec<&SmolStr> {
	/// Like [`Vec::<String>::join`] for [`SmolStr`]
	fn join(&self, sep: &str) -> SmolStr {
		self.iter()
			.map(|s| s.as_str())
			.collect::<Vec<_>>()
			.join(sep)
			.into()
	}
}
