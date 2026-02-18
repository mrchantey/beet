use crate::prelude::*;
use beet_core::prelude::*;

/// Create a routable tool that can be called with [`Request`]/[`Response`]
/// pairs. The request body is deserialized to `In` and the `Out` is
/// serialized back into the response body.
///
/// Unlike [`tool`], this constructor:
/// - Inserts a [`PathPartial`] from the provided `path`
/// - Spawns the inner typed tool as a [`RouteHidden`] child entity
/// - Adds a [`RouteToolMarker`] so the entity is recognized as a
///   route tool
///
/// Content-type negotiation is based on the request's `content-type` header:
/// - `application/json` (default): uses `serde_json`
/// - `application/x-postcard`: uses `postcard` binary serialization
///
/// The response `content-type` mirrors the request format.
///
/// ## Example
///
/// ```
/// # use beet_stack::prelude::*;
/// # use beet_core::prelude::*;
///
/// // Create a route tool from a typed handler
/// let bundle = route_tool("add", |(a, b): (i32, i32)| -> i32 { a + b });
///
/// // Can be called with Request/Response
/// let request = Request::with_json("/add", &(2i32, 2i32)).unwrap();
/// let response = AsyncPlugin::world()
///     .spawn(bundle)
///     .call_blocking::<Request, Response>(request)
///     .unwrap();
///
/// let result: i32 = response.deserialize_blocking().unwrap();
/// assert_eq!(result, 4);
/// ```
///
/// ## Using with serialized request/response
///
/// Route tools support serialized request/response calls,
/// using the `content-type` header for format negotiation:
///
/// ```
/// # use beet_stack::prelude::*;
/// # use beet_core::prelude::*;
/// # use serde::{Serialize, Deserialize};
/// #[derive(Debug, Serialize, Deserialize, Reflect)]
/// struct AddInput { a: i32, b: i32 }
///
/// let request = Request::with_json("/", &AddInput { a: 10, b: 20 }).unwrap();
/// let response = AsyncPlugin::world()
///     .spawn(route_tool("add", |input: AddInput| -> i32 { input.a + input.b }))
///     .call_blocking::<Request, Response>(request)
///     .unwrap();
///
/// let result: i32 = response.deserialize_blocking().unwrap();
/// assert_eq!(result, 30);
/// ```
pub fn route_tool<H: 'static, M>(
	path: impl AsRef<std::path::Path>,
	handler: H,
) -> impl Bundle
where
	H: IntoToolHandler<M>,
	H::In: 'static + Send + Sync + serde::de::DeserializeOwned,
	H::Out: 'static + Send + Sync + serde::Serialize,
{
	(
		PathPartial::new(path),
		ToolMeta::of::<H, H::In, H::Out>(),
		RouteToolMarker,
		route_tool_handler::<H::In, H::Out>(),
		OnSpawn::insert_child((RouteHidden, tool(handler))),
	)
}

/// Marker component indicating this entity is a route tool that bridges
/// [`Request`]/[`Response`] calls to an inner typed child tool.
#[derive(Component)]
pub struct RouteToolMarker;

/// Creates the [`ToolHandler<Request, Response>`] that bridges
/// `Request`/`Response` tool calls to the inner typed child tool.
///
/// When called:
/// 1. The request body is consumed asynchronously to support streaming
/// 2. The request's `content-type` header determines the [`ExchangeFormat`]
/// 3. The body bytes are deserialized to `In`
/// 4. The first child entity is called with typed `In`/`Out`
/// 5. The `Out` is serialized into a [`Response`] body
/// 6. The response is delivered via the out handler
fn route_tool_handler<Input, Output>() -> ToolHandler<Request, Response>
where
	Input: 'static + Send + Sync + serde::de::DeserializeOwned,
	Output: 'static + Send + Sync + serde::Serialize,
{
	ToolHandler::new(
		move |ToolCall {
		          mut commands,
		          tool,
		          input: request,
		          out_handler,
		      }: ToolCall<Request, Response>| {
			// determine serialization format from request content-type
			let format = ExchangeFormat::from_content_type(
				request.get_header("content-type"),
			)?;

			commands.run(async move |world: AsyncWorld| -> Result {
				// consume body asynchronously to support streaming
				let body_bytes = request.body.into_bytes().await?;
				let input: Input = format.deserialize(&body_bytes)?;

				// get the first child entity (the inner tool)
				let child = world
					.entity(tool)
					.get(|children: &Children| children[0])
					.await?;

				// call the inner tool with typed args
				let output: Output =
					world.entity(child).call::<Input, Output>(input).await?;

				// serialize result into a response
				let body_bytes = format.serialize(&output)?;
				let response = Response::ok()
					.with_content_type(format.content_type_str())
					.with_body(body_bytes);

				// deliver response via the out handler using fresh
				// AsyncCommands obtained through SystemState
				world
					.with_then(move |world: &mut World| -> Result {
						let result = {
							let mut state = bevy::ecs::system::SystemState::<
								AsyncCommands,
							>::new(world);
							let async_commands = state.get_mut(world);
							let result =
								out_handler.call(async_commands, response);
							state.apply(world);
							result
						};
						world.flush();
						result
					})
					.await
			});
			Ok(())
		},
	)
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;

	#[derive(Debug, Serialize, Deserialize, Reflect, PartialEq)]
	struct AddInput {
		a: i32,
		b: i32,
	}

	fn add_route_tool() -> impl Bundle {
		route_tool("add", |input: AddInput| -> i32 { input.a + input.b })
	}

	// -- Request/Response JSON round-trip --

	#[test]
	fn json_request_response() {
		let request =
			Request::with_json("/add", &AddInput { a: 10, b: 20 }).unwrap();

		let response = AsyncPlugin::world()
			.spawn(add_route_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap();

		response.status().xpect_eq(StatusCode::Ok);
		response
			.get_header("content-type")
			.unwrap()
			.xpect_eq("application/json");
		let result: i32 = response.deserialize_blocking().unwrap();
		result.xpect_eq(30);
	}

	// -- JSON is the default when no content-type is specified --

	#[test]
	fn json_default_content_type() {
		let request = Request::post("/add").with_body(r#"{"a":1,"b":2}"#);

		AsyncPlugin::world()
			.spawn(add_route_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap()
			.deserialize_blocking::<u32>()
			.unwrap()
			.xpect_eq(3);
	}

	// -- postcard binary round-trip --

	#[test]
	fn postcard_request_response() {
		let request =
			Request::with_postcard("/add", &AddInput { a: 5, b: 7 }).unwrap();

		let response = AsyncPlugin::world()
			.spawn(add_route_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap();

		response
			.get_header("content-type")
			.unwrap()
			.xpect_eq("application/x-postcard");

		let result: i32 = response.deserialize_blocking().unwrap();
		result.xpect_eq(12);
	}

	// -- unit input with empty body --

	#[test]
	fn unit_input_empty_body() {
		let response = AsyncPlugin::world()
			.spawn(route_tool("unit", || -> i32 { 42 }))
			.call_blocking::<Request, Response>(Request::get("/"))
			.unwrap();

		let result: i32 = response.deserialize_blocking().unwrap();
		result.xpect_eq(42);
	}

	// -- ExchangeFormat unit tests --

	#[test]
	fn format_from_content_type() {
		ExchangeFormat::from_content_type(None)
			.unwrap()
			.xpect_eq(ExchangeFormat::Json);
		ExchangeFormat::from_content_type(Some("application/json"))
			.unwrap()
			.xpect_eq(ExchangeFormat::Json);
		ExchangeFormat::from_content_type(Some(
			"application/json; charset=utf-8",
		))
		.unwrap()
		.xpect_eq(ExchangeFormat::Json);
		ExchangeFormat::from_content_type(Some("application/x-postcard"))
			.unwrap()
			.xpect_eq(ExchangeFormat::Postcard);

		// unrecognized content-type should error
		ExchangeFormat::from_content_type(Some("text/plain")).xpect_err();
	}

	#[test]
	fn format_roundtrip_json() {
		let input = AddInput { a: 1, b: 2 };
		let bytes = ExchangeFormat::Json.serialize(&input).unwrap();
		let output: AddInput =
			ExchangeFormat::Json.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}

	#[test]
	fn format_roundtrip_postcard() {
		let input = AddInput { a: 3, b: 4 };
		let bytes = ExchangeFormat::Postcard.serialize(&input).unwrap();
		let output: AddInput =
			ExchangeFormat::Postcard.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}

	// -- Response::deserialize round-trip --

	#[test]
	fn response_deserialize_json_roundtrip() {
		let request =
			Request::with_json("/add", &AddInput { a: 10, b: 5 }).unwrap();

		AsyncPlugin::world()
			.spawn(add_route_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap()
			.deserialize_blocking::<i32>()
			.unwrap()
			.xpect_eq(15);
	}

	#[test]
	fn response_deserialize_postcard_roundtrip() {
		let request =
			Request::with_postcard("/add", &AddInput { a: 3, b: 9 }).unwrap();

		AsyncPlugin::world()
			.spawn(add_route_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap()
			.deserialize_blocking::<i32>()
			.unwrap()
			.xpect_eq(12);
	}

	// -- with_json_str convenience --

	#[test]
	fn json_str_request() {
		let request = Request::with_json_str("/add", r#"{"a":100,"b":200}"#);

		AsyncPlugin::world()
			.spawn(add_route_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap()
			.deserialize_blocking::<i32>()
			.unwrap()
			.xpect_eq(300);
	}

	// -- route tool inserts path partial --

	#[test]
	fn inserts_path_partial() {
		let mut world = RouterPlugin::world();
		world
			.spawn(add_route_tool())
			.get::<PathPattern>()
			.unwrap()
			.annotated_route_path()
			.to_string()
			.xpect_eq("/add");
	}
}
