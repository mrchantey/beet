use beet_core::prelude::*;

/// A tool is a typed Endpoint
pub trait Tool {
	/// The input type for the tool
	type In = ();
	/// The output type for the tool
	type Out = ();

	/// Run the tool
	fn call(
		entity: AsyncEntity,
		input: Self::In,
	) -> impl Future<Output = Result<Self::Out>>;
}
