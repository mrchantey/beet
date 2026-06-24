use crate::prelude::*;
use beet::prelude::*;

/// A server action: adds two numbers. Invoked through the generated, typed client
/// caller in `codegen/client_actions.rs`.
pub async fn post(cx: ActionContext<Json<AddArgs>>) -> Result<i32> {
	let AddArgs { a, b } = cx.input.0;
	Ok(a + b)
}
