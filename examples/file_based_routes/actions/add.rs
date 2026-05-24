use crate::prelude::*;

/// A server action: adds two numbers. Invoked from the generated client caller
/// in `codegen/client_actions.rs`.
pub async fn post(cx: ActionContext<Json<AddArgs>>) -> Result<i32> {
	let AddArgs { a, b } = cx.input.0;
	Ok(a + b)
}
