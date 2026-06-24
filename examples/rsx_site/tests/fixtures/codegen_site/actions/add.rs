use beet::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
pub struct AddArgs {
	pub a: i32,
	pub b: i32,
}

/// Adds two numbers, called via the generated client caller.
pub async fn post(cx: ActionContext<Json<AddArgs>>) -> Result<i32> {
	let args = cx.input.0;
	Ok(args.a + args.b)
}
