use crate::prelude::*;
use extend::ext;

/// String extensions
#[ext]
pub impl<T: AsRef<str>> T {
	/// Skip empty lines, while preserving
	/// whitespace in the first non-empty line.
	fn trim_start_lines(&self) -> String {
		self.as_ref()
			.lines()
			.skip_while(|line| line.trim().is_empty())
			.collect::<Vec<_>>()
			.join("\n")
			.into()
	}
	/// Skip trailing empty lines, while preserving
	/// whitespace in the last non-empty pines
	fn trim_end_lines(&self) -> String {
		let lines: Vec<_> = self.as_ref().lines().collect();
		lines
			.iter()
			.rev()
			.skip_while(|line| line.trim().is_empty())
			.collect::<Vec<_>>()
			.iter()
			.rev()
			.map(|s| **s)
			.collect::<Vec<_>>()
			.join("\n")
			.into()
	}
}


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
