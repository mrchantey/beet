use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;


/// A component that allows streaming bytes to the response body.
///
/// This component is added to the agent entity when using [`flow_exchange_stream`].
/// Dropping this component (e.g. by despawning the entity) will close the stream.
#[derive(Component, Clone)]
pub struct BodyStream(pub async_channel::Sender<Result<bytes::Bytes>>);

impl BodyStream {
	/// Send bytes to the stream.
	pub async fn send_bytes(
		&self,
		bytes: impl Into<bytes::Bytes>,
	) -> Result<()> {
		self.0
			.send(Ok(bytes.into()))
			.await
			.map_err(|_| bevyhow!("Stream closed"))
	}

	/// Send text to the stream.
	pub async fn send_text(&self, text: impl Into<String>) -> Result<()> {
		self.send_bytes(text.into().into_bytes()).await
	}

	/// Send a Server-Sent Event (SSE).
	pub async fn send_sse(
		&self,
		event: impl AsRef<str>,
		data: impl AsRef<str>,
	) -> Result<()> {
		let msg =
			format!("event: {}\ndata: {}\n\n", event.as_ref(), data.as_ref());
		self.send_text(msg).await
	}

	/// Send a Server-Sent Event (SSE) with JSON data.
	#[cfg(feature = "serde")]
	pub async fn send_sse_json<T: serde::Serialize>(
		&self,
		event: impl AsRef<str>,
		data: &T,
	) -> Result<()> {
		let json = serde_json::to_string(data)?;
		self.send_sse(event, &json).await
	}
}

/// Creates a streaming exchange handler.
///
/// Unlike [`flow_exchange`], this handler sends the response headers immediately
/// and provides a [`BodyStream`] component to the agent for streaming the body.
///
/// Default response status is `200 OK`.
pub fn flow_exchange_stream(func: impl BundleFunc) -> impl Bundle {
	flow_exchange_stream_with(func, |_| {})
}

/// Creates a streaming exchange handler configured for Server-Sent Events (SSE).
///
/// Sets `Content-Type: text/event-stream` and `Cache-Control: no-cache`.
pub fn flow_exchange_sse(func: impl BundleFunc) -> impl Bundle {
	flow_exchange_stream_with(func, |res| {
		res.parts
			.parts_mut()
			.insert_header("content-type", "text/event-stream");
		res.parts
			.parts_mut()
			.insert_header("cache-control", "no-cache");
	})
}

/// Creates a streaming exchange handler with custom response modification.
///
/// The `modify` closure is called on the response before it is sent (but after the body stream is attached).
/// Use this to set status codes or headers.
pub fn flow_exchange_stream_with(
	func: impl BundleFunc,
	modify: impl 'static + Send + Sync + Clone + Fn(&mut Response),
) -> impl Bundle {
	OnSpawn::observe(
		move |ev: On<ExchangeStart>, mut commands: Commands| -> Result {
			let server_entity = ev.event_target();
			let (req, cx) = ev.take()?;

			let (sender, receiver) = async_channel::bounded(1);
			let mut response = Response::ok().with_body(Body::stream(receiver));

			// Allow customization of headers/status
			modify(&mut response);

			let status = response.status();

			// Send headers immediately
			cx.end_no_entity(response)?;

			// Spawn the agent entity with BodyStream
			let agent = commands
				.spawn((
					Name::new("Flow Exchange Stream Agent"),
					ChildOf(server_entity),
					req,
					cx,
					BodyStream(sender),
					ResponseStatus(status),
				))
				.id();

			// Spawn the action root
			commands.spawn((
				Name::new("Flow Exchange Stream Action"),
				ChildOf(agent),
				ActionOf(agent),
				OnSpawn::observe(stream_outcome_handler),
				func.clone().bundle_func(),
				OnSpawn::trigger(GetOutcome),
			));

			Ok(())
		},
	)
}

#[derive(Component)]
struct ResponseStatus(StatusCode);

/// Handles outcome events for streaming exchanges.
/// Cleans up the agent and triggers ExchangeEnd.
fn stream_outcome_handler(
	ev: On<Outcome>,
	agents: AgentQuery,
	mut commands: Commands,
	query: Query<(&ExchangeContext, &ResponseStatus)>,
) {
	let action = ev.target();
	let agent = agents.entity(action);

	if let Ok((cx, status)) = query.get(agent) {
		// Log the completion
		commands.trigger(ExchangeEnd {
			entity: agent,
			start_time: cx.start_time,
			status: status.0,
		});
	}

	// Despawn agent to drop BodyStream and close the channel
	commands.entity(agent).despawn();
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;

	async fn poll_once_and_update<T>(
		app: &mut App,
		mut future: impl Future<Output = T> + Unpin,
	) -> T {
		loop {
			if let Some(val) =
				beet_core::exports::futures_lite::future::poll_once(&mut future)
					.await
			{
				return val;
			}
			app.update();
			time_ext::sleep_millis(10).await;
		}
	}

	fn stream_chunk(text: &'static str) -> impl Bundle {
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      agents: AgentQuery,
			      mut commands: AsyncCommands| {
				let action = ev.target();
				let agent = agents.entity(action);
				let text = text.to_string();
				commands.run(move |world| async move {
					let stream = world
						.entity(agent)
						.get_cloned::<BodyStream>()
						.await
						.unwrap();
					stream.send_text(text).await.unwrap();
					world.entity(action).with(|mut e| {
						e.trigger_target(Outcome::Pass);
					});
				});
			},
		)
	}

	#[beet_core::test(timeout_ms = 1000)]
	async fn stream_works() {
		let mut app = App::new();
		app.add_plugins((ControlFlowPlugin, AsyncPlugin));
		let response = app
			.world_mut()
			.spawn(flow_exchange_stream(|| {
				// Use a sequence to send multiple chunks
				(Sequence, children![
					stream_chunk("hello"),
					stream_chunk(" world")
				])
			}))
			.exchange(Request::get("foo"))
			.await;

		response.status().xpect_eq(StatusCode::Ok);

		poll_once_and_update(&mut app, Box::pin(response.text()))
			.await
			.unwrap()
			.xpect_eq("hello world");
	}

	#[beet_core::test(timeout_ms = 1000)]
	async fn sse_works() {
		let mut app = App::new();
		app.add_plugins((ControlFlowPlugin, AsyncPlugin));
		let response = app
			.world_mut()
			.spawn(flow_exchange_sse(|| {
				OnSpawn::observe(
					|ev: On<GetOutcome>,
					 agents: AgentQuery,
					 mut commands: AsyncCommands| {
						let action = ev.target();
						let agent = agents.entity(action);
						commands.run(move |world| async move {
							world
								.entity(agent)
								.get_cloned::<BodyStream>()
								.await
								.unwrap()
								.send_sse("test", "data")
								.await
								.unwrap();
							world.entity(action).with(|mut e| {
								e.trigger_target(Outcome::Pass);
							});
						});
					},
				)
			}))
			.exchange(Request::get("foo"))
			.await;

		response.status().xpect_eq(StatusCode::Ok);
		response
			.header_contains(http::header::CONTENT_TYPE, "text/event-stream")
			.xpect_true();

		poll_once_and_update(&mut app, Box::pin(response.text()))
			.await
			.unwrap()
			.xpect_contains("event: test")
			.xpect_contains("data: data");
	}
}
