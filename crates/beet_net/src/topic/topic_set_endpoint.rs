use crate::prelude::*;
use anyhow::Result;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::RwLock;


/// A reactive wrapper around a [`TopicSet`] that broadcasts changes
#[derive(Debug, Clone)]
pub struct TopicSetEndpoint {
	topic_set: Arc<RwLock<TopicSet>>,
	/// For whenever this relay's pubs or subs gets updated, used by `self` to broadcast changes
	on_change_pub: Publisher<TopicSet>,
	/// For whenever this relay's pubs or subs gets updated, use by others to listen for changes
	on_change_sub: Subscriber<TopicSet>,
}

impl TopicSetEndpoint {
	pub fn new(channel_map: &mut TopicChannelMap) -> Self {
		let on_change_pub = channel_map.unchecked_publisher(
			Self::on_change_topic_address(),
			TopicMethod::Update,
		);
		let on_change_sub = channel_map.unchecked_subscriber(
			Self::on_change_topic_address(),
			TopicMethod::Update,
		);

		Self {
			topic_set: Default::default(),
			on_change_pub,
			on_change_sub,
		}
	}

	pub fn topic_set(&self) -> impl Deref<Target = TopicSet> + '_ {
		self.topic_set.read().unwrap()
	}

	pub fn on_change_topic_address() -> TopicAddress {
		TopicAddress::new("relay/topic_graph")
	}
	pub fn on_change_topic() -> Topic {
		Topic::new(
			Self::on_change_topic_address(),
			TopicScheme::PubSub,
			TopicMethod::Update,
		)
	}
	pub fn on_change(&self) -> Subscriber<TopicSet> {
		self.on_change_sub.clone()
	}

	/// Adds a publisher and triggers on_change if it didnt already exist
	pub fn add_publisher(
		&self,
		topic: &Topic,
		payload_type: &PayloadType,
	) -> Result<()> {
		let mut graph = self.topic_set.write().unwrap();

		if graph.try_add_publisher(topic, payload_type)? {
			self.on_change_pub.send(&graph)?;
		}
		Ok(())
	}

	/// Adds a subscriber and triggers on_change if it didnt already exist
	pub fn add_subscriber(
		&self,
		topic: &Topic,
		payload_type: &PayloadType,
	) -> Result<()> {
		let mut graph = self.topic_set.write().unwrap();

		if graph.try_add_subscriber(topic, payload_type)? {
			self.on_change_pub.send(&graph)?;
		};

		Ok(())
	}

	/// Provides full mutable access to the graph through a callback
	/// and triggers on_change
	pub fn mutate(&mut self, f: impl FnOnce(&mut TopicSet)) -> Result<()> {
		let mut graph = self.topic_set.write().unwrap();
		f(&mut graph);
		self.on_change_pub.send(&graph)?;
		Ok(())
	}
}
