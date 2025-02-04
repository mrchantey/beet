use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

/// File location of an rsx macro, used by [RsxTemplate]
/// to reconcile rsx nodes with html partials
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RsxLocation {
	/// in the macro this is set via file!(),
	/// in the cli its set via the file path,
	/// when setting this it must be in the same
	/// format as file!() would return
	pub file: String,
	pub line: usize,
	pub col: usize,
}
impl RsxLocation {
	pub fn new(file: impl Into<String>, line: usize, col: usize) -> Self {
		Self {
			file: file.into(),
			line,
			col,
		}
	}

	pub fn file(&self) -> &str { &self.file }
	pub fn line(&self) -> usize { self.line }
	pub fn col(&self) -> usize { self.col }
}



/// the 'RsxLocation' of individual effects, used by [RsxTemplateNode]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineColumn {
	pub line: u32,
	pub column: u32,
}


impl LineColumn {
	pub fn new(line: u32, column: u32) -> Self { Self { line, column } }
	pub fn to_hash(&self) -> u64 {
		let mut hasher = DefaultHasher::new();
		self.hash(&mut hasher);
		hasher.finish()
	}
}
