use crate::prelude::*;
use anyhow::Result;
use std::ops::Deref;
pub type DomainId = u64;


/// A relay for pubsub messaging, in dds terms this is a participant
// Doesnt look like much here, but has implementations in `relay_pubsub.rs` and `relay_request.rs`
#[derive(Debug, Clone)]
pub struct Relay {
	domain_id: DomainId,
	/// The reactive list of interests and intents
	pub(crate) endpoint: TopicSetEndpoint,
	/// The map of topics to message channels
	pub(crate) channel_map: TopicChannelMap,
}

impl Relay {
	pub fn new(domain_id: DomainId) -> Self {
		let mut this = Self::default();
		this.domain_id = domain_id;
		this
	}
	pub fn domain_id(&self) -> DomainId { self.domain_id }
	// pub fn channel_map(&self) -> &TopicChannelMap { &self.channel_map }

	pub fn topic_set_changed(&self) -> Subscriber<TopicSet> {
		self.endpoint.on_change()
	}
	pub fn topic_set(&self) -> impl Deref<Target = TopicSet> + '_ {
		self.endpoint.topic_set()
	}
	pub fn get_all_messages(&mut self) -> Result<Vec<StateMessage>> {
		self.channel_map.get_all_messages()
	}
	pub async fn try_send_all_messages(
		&mut self,
		messages: Vec<StateMessage>,
	) -> Result<Vec<StateMessage>> {
		self.channel_map.try_send_all_messages(messages).await
	}
	pub async fn sync_local(&mut self, b2: &mut Self) -> Result<()> {
		let msg1 = self.get_all_messages()?;
		let msg2 = b2.get_all_messages()?;
		self.try_send_all_messages(msg2).await?;
		b2.try_send_all_messages(msg1).await?;
		Ok(())
	}
}

impl Default for Relay {
	fn default() -> Self {
		let mut channel_map = TopicChannelMap::default();

		let endpoint = TopicSetEndpoint::new(&mut channel_map);

		Self {
			domain_id: 0,
			endpoint,
			channel_map,
		}
	}
}

// impl Drop for RelayInner {
// 	fn drop(&mut self) {
// 		// spawn a thread to join the handles?
// 		#[cfg(not(target_arch = "wasm32"))]
// 		for join_handle in self.join_handles.lock().drain(..) {
// 			join_handle.join().unwrap();
// 		}
// 	}
// }
