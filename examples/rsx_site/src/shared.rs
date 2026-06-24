//! Types shared between server-action handlers and their generated client callers.
use beet::prelude::*;

/// Arguments for the `add` server action.
#[derive(Debug, Serialize, Deserialize)]
pub struct AddArgs {
	/// First addend.
	pub a: i32,
	/// Second addend.
	pub b: i32,
}
