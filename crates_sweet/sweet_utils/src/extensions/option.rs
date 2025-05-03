use anyhow::*;
use extend::ext;

#[ext]
pub impl<T> Option<T> {
	fn or_err(self) -> Result<T> {
		match self {
			Some(value) => Ok(value),
			None => Err(anyhow!("Expected Some")),
		}
	}
}
