use beet_core::prelude::*;
use eventsource_stream::EventStream;
use eventsource_stream::Eventsource;

#[extend::ext(name=ResponseClientExt)]
pub impl Response {
	/// Server Sent Events, checks for Ok status before parsing
	#[allow(async_fn_in_trait)]
	async fn event_source(self) -> Result<EventStream<Body>> {
		self.into_result().await?.body.eventsource().xok()
	}
}


#[cfg(all(
	test,
	any(
		target_arch = "wasm32",
		all(
			feature = "native-tls",
			any(feature = "reqwest", feature = "ureq")
		)
	)
))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::tasks::futures_lite::StreamExt;

	// TODO demonstrate deserializing the actual event
	// #[derive(serde::Deserialize, Debug)]
	// struct TestEvent {
	// 	testing: bool,
	// 	sse_dev: String,
	// 	msg: String,
	// 	now: u64,
	// }

	#[cfg_attr(feature = "reqwest", beet_core::test(tokio))]
	#[cfg_attr(not(feature = "reqwest"), beet_core::test)]
	// TODO spin up our own server for tests
	// #[ignore = "hits network"]
	async fn works() {
		let mut ev = Request::get("https://sse.dev/test")
			.send()
			.await
			.unwrap()
			.event_source()
			.await
			.unwrap();

		let mut count = 0;
		while let Some(Ok(event)) = ev.next().await {
			event.data.xref().xpect_contains("It works!");
			if count == 2 {
				break;
			} else {
				count += 1;
			}
		}
	}
}
