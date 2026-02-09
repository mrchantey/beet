use crate::prelude::*;
use beet_core::prelude::*;



pub struct InterfaceToolIn<Payload, Params> {
	pub payload: Payload,
	/// The parameters of the tool call
	pub params: Params,
	/// The dynamic path segments of this route,
	/// according to its [`PathPattern`]
	pub path_segments: MultiMap<String, String>,
}
