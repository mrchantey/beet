use crate::bevyhow;
use bevy::prelude::*;
use extend::ext;

#[ext]
pub impl<T> Option<T> {
	fn or_err(self) -> Result<T> {
		match self {
			Some(value) => Ok(value),
			None => Err(bevyhow!("Expected Some")),
		}
	}
}
