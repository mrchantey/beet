use crate::prelude::*;
use beet_core::prelude::*;




/// Flattens an error returned by a tool into a tool call error,
/// it is still propagated to the caller but without the double unwrap.
pub fn flatten<T>(
	ToolCall {
		commands,
		input,
		out_handler,
		..
	}: ToolCall<Result<T>, T>,
) -> Result {
	out_handler.call(commands, input?)
}
