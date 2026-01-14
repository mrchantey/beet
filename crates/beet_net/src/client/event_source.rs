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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::tasks::futures_lite::StreamExt;

	#[beet_core::test]
	// TODO spin up our own server for tests
	#[ignore = "hits network"]
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
