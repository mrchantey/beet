use crate::prelude::*;
use bevy::prelude::*;
use eventsource_stream::EventStream;
use eventsource_stream::Eventsource;


impl Response {
	/// Server Sent Events, checks for Ok status before parsing
	pub async fn event_source(self) -> Result<EventStream<Body>> {
		self.into_result().await?.body.eventsource().xok()
	}
}

#[cfg(feature = "native-tls")]
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::tasks::futures_lite::StreamExt;
	use sweet::prelude::*;

	#[sweet::test]
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
			event.data.xref().xpect().to_contain("It works!");
			if count == 2 {
				break;
			} else {
				count += 1;
			}
		}

		// .xmap(|res| res.status())
		// .xpect_eq(200);
	}
}
