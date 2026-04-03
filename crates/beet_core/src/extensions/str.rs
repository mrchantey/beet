use crate::prelude::*;
use extend::ext;



// #[ext]
// pub impl SmolStr {

// 	pub fn

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
