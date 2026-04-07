use crate::prelude::*;
use beet_core::prelude::*;
use bevy::tasks::futures_lite::StreamExt;
use eventsource_stream::Event;
use eventsource_stream::EventStream;
use eventsource_stream::Eventsource;
use futures::Stream;
use send_wrapper::SendWrapper;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

/// A parsed Server-Sent Event with typed data.
///
/// SSE events contain an event type (defaulting to "message") and a data payload.
/// This struct provides strongly-typed access to the deserialized data.
///
/// # Example
/// ```ignore
/// #[derive(serde::Deserialize)]
/// struct MyEvent {
///     id: u64,
///     message: String,
/// }
///
/// let mut stream = response.event_source_typed::<MyEvent>().await?;
/// while let Some(Ok(sse)) = stream.next().await {
///     println!("Event type: {}, data: {:?}", sse.event, sse.data);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SseBody<T> {
	/// The event type, defaults to "message" if not specified by the server
	pub event: String,
	/// The deserialized event data
	pub data: T,
}

impl<T> SseBody<T> {
	/// Creates a new `SseBody` with the given event type and data
	pub fn new(event: impl Into<String>, data: T) -> Self {
		Self {
			event: event.into(),
			data,
		}
	}
}

/// A stream of mapped SSE events.
///
/// This stream wraps an underlying `EventStream` and applies a mapping function
/// to each event, allowing for custom deserialization or transformation.
pub struct SseStream<S> {
	inner: SendWrapper<Pin<Box<S>>>,
}

impl<S> SseStream<S> {
	fn new(stream: S) -> Self {
		Self {
			inner: SendWrapper::new(Box::pin(stream)),
		}
	}
}

impl<S, T> Stream for SseStream<S>
where
	S: Stream<Item = T>,
{
	type Item = T;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Self::Item>> {
		// SAFETY: We're not moving the inner stream, just polling it
		let this = self.get_mut();
		this.inner.as_mut().poll_next(cx)
	}
}

/// Extension trait for [`Response`] that provides Server-Sent Events (SSE) streaming.
///
/// This trait adds methods to convert an HTTP response into various SSE stream types,
/// supporting raw event access, custom mapping, and automatic JSON deserialization.
#[extend::ext(name = ResponseSseExt)]
pub impl Response {
	/// Returns a raw SSE event stream, checking for Ok status before parsing.
	///
	/// This returns the underlying `EventStream` from `eventsource-stream`,
	/// providing access to raw `Event` objects with string data.
	///
	/// # Example
	/// ```ignore
	/// let mut stream = response.event_source_raw().await?;
	/// while let Some(Ok(event)) = stream.next().await {
	///     println!("Event: {}, Data: {}", event.event, event.data);
	/// }
	/// ```
	#[allow(async_fn_in_trait)]
	async fn event_source_raw(self) -> Result<EventStream<Body>> {
		self.into_result().await?.body.eventsource().xok()
	}

	/// Returns a mapped SSE event stream with custom per-event transformation.
	///
	/// This method allows you to provide a custom function that transforms each
	/// raw `Event` into your desired type. This is useful when you need custom
	/// deserialization logic or want to handle different event types differently.
	///
	/// # Arguments
	/// * `func` - A function that takes a raw `Event` and returns a `Result<SseBody<T>>`
	///
	/// # Example
	/// ```ignore
	/// let stream = response.event_source_mapped(|event| {
	///     let data: MyType = serde_json::from_str(&event.data)?;
	///     Ok(SseBody::new(event.event, data))
	/// }).await?;
	/// ```
	#[allow(async_fn_in_trait)]
	async fn event_source_mapped<T, F>(
		self,
		func: F,
	) -> Result<SseStream<impl Stream<Item = Result<SseBody<T>>>>>
	where
		F: 'static + Fn(Event) -> Result<SseBody<T>> + Send + Sync,
	{
		let raw_stream = self.event_source_raw().await?;
		let mapped = raw_stream.map(move |result| match result {
			Ok(event) => func(event),
			Err(err) => Err(bevyhow!("SSE parse error: {}", err)),
		});
		SseStream::new(mapped).xok()
	}

	/// Returns a typed SSE event stream with automatic JSON deserialization.
	///
	/// Each event's data field is deserialized as JSON into the specified type `T`.
	/// The event type string is preserved in the resulting `SseBody`.
	///
	/// # Type Parameters
	/// * `T` - The type to deserialize each event's data into. Must implement `serde::Deserialize`.
	///
	/// # Example
	/// ```ignore
	/// #[derive(serde::Deserialize)]
	/// struct ChatMessage {
	///     user: String,
	///     text: String,
	/// }
	///
	/// let mut stream = response.event_source_typed::<ChatMessage>().await?;
	/// while let Some(Ok(sse)) = stream.next().await {
	///     println!("{}: {}", sse.data.user, sse.data.text);
	/// }
	/// ```
	#[cfg(feature = "json")]
	#[allow(async_fn_in_trait)]
	async fn event_source_typed<T>(
		self,
	) -> Result<SseStream<impl Stream<Item = Result<SseBody<T>>>>>
	where
		T: 'static + serde::de::DeserializeOwned + Send + Sync,
	{
		self.event_source_mapped(|event| {
			let data: T = serde_json::from_str(&event.data).map_err(|err| {
				bevyhow!(
					"Failed to deserialize SSE event data: {}\nRaw data: {}",
					err,
					event.data
				)
			})?;
			SseBody::new(event.event, data).xok()
		})
		.await
	}
}


#[cfg(test)]
#[cfg(all(
	feature = "server",
	feature = "ureq",
	feature = "json",
	not(target_arch = "wasm32")
))]
mod test {
	use super::*;

	#[beet_core::test]
	async fn raw_works() {
		let server = EchoHttpServer::new().await;
		let mut ev = Request::get(server.url().clone().push("sse"))
			.with_param("count", "3")
			.send()
			.await
			.unwrap()
			.event_source_raw()
			.await
			.unwrap();

		let mut count = 0;
		while let Some(Ok(event)) = ev.next().await {
			event.data.xref().xpect_contains("hello");
			count += 1;
			if count >= 3 {
				break;
			}
		}
		count.xpect_eq(3);
	}

	#[beet_core::test]
	async fn typed_works() {
		let server = EchoHttpServer::new().await;
		let mut ev = Request::get(server.url().clone().push("sse"))
			.with_param("count", "3")
			.send()
			.await
			.unwrap()
			.event_source_typed::<SseTestEvent>()
			.await
			.unwrap();

		let mut count = 0;
		while let Some(Ok(sse)) = ev.next().await {
			sse.data.index.xpect_eq(count as u32);
			sse.data.msg.xpect_eq("hello");
			count += 1;
			if count >= 3 {
				break;
			}
		}
		count.xpect_eq(3);
	}

	#[beet_core::test]
	async fn mapped_works() {
		let server = EchoHttpServer::new().await;
		let mut ev = Request::get(server.url().clone().push("sse"))
			.with_param("count", "3")
			.send()
			.await
			.unwrap()
			.event_source_mapped(|event| {
				let data: serde_json::Value =
					serde_json::from_str(&event.data)?;
				let msg = data["msg"].as_str().unwrap_or_default().to_string();
				SseBody::new(event.event, msg).xok()
			})
			.await
			.unwrap();

		let mut count = 0;
		while let Some(Ok(sse)) = ev.next().await {
			sse.data.xpect_eq("hello");
			count += 1;
			if count >= 3 {
				break;
			}
		}
		count.xpect_eq(3);
	}
}
