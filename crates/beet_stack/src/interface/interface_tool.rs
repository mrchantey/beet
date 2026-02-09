use beet_core::prelude::*;

/// The deserialized form of a [`Request`]
pub struct InterfaceToolIn<Payload, Params> {
	pub payload: Payload,
	/// The parameters of the tool call
	pub params: Params,
	/// The dynamic path segments of this route,
	/// according to its [`PathPattern`]
	pub path_segments: MultiMap<String, String>,
}





#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn my_tool() -> impl Bundle {
		(PathPartial::new("add"), tool(|req: Request| req.mirror()))
	}


	#[test]
	fn works() {
		World::new()
			.spawn(my_tool())
			.call_blocking::<_, Response>(Request::get("foo"))
			.unwrap()
			.xpect_eq(Response::ok());
	}
}
