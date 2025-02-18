/// File location of an rsx macro, used by [RsxTemplate]
/// to reconcile rsx nodes with html partials
///
/// ```rust ignore
/// # use beet_rsx_macros::rsx;
/// let tree = rsx!{<div>hello</div>};
/// //              ^ this location
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RsxMacroLocation {
	/// in the macro this is set via file!(),
	/// in the cli its set via the file path,
	/// when setting this it must be in the same
	/// format as file!() would return
	pub file: String,
	pub line: usize,
	pub col: usize,
}
impl Default for RsxMacroLocation {
	fn default() -> Self {
		Self {
			file: "placeholder".to_string(),
			line: 0,
			col: 0,
		}
	}
}

impl RsxMacroLocation {
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
