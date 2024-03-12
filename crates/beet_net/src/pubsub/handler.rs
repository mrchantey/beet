use crate::prelude::*;
use anyhow::Result;




pub trait TopicHandler<T: Payload> {
	fn topic() -> Topic;
	fn subscriber(relay: &mut Relay) -> Result<Subscriber<T>> {
		relay.add_subscriber_with_topic(Self::topic())
	}

	fn publisher(relay: &mut Relay) -> Result<Publisher<T>> {
		relay.add_publisher_with_topic(Self::topic())
	}
}
