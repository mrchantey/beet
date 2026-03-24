use crate::prelude::*;
use beet_core::prelude::*;


pub struct Topic<T: Document> {
	pub path: String,
	/// The type name of the document this topic is for
	pub doc_type: String,
	/// Optional display name for the topic, e.g. "Alice's Messages"
	pub display_name: Option<String>,
	phantom: PhantomData<T>,
}

impl<T: Document> Default for Topic<T> {
	fn default() -> Self { Self::new(Uuid7::<Self>::default().into_string()) }
}

impl<T: Document> Topic<T> {
	pub fn new(id: String) -> Self {
		Self {
			path: id,
			doc_type: std::any::type_name::<T>().into(),
			display_name: None,
			phantom: PhantomData,
		}
	}
	pub fn new_from(id: impl TopicKey) -> Self { Self::new(id.topic_path()) }

	pub fn with_name(mut self, name: String) -> Self {
		self.display_name = Some(name);
		self
	}
}

impl<T: Document> Document for Topic<T> {
	type Id = String;
	fn id(&self) -> Self::Id { self.path.clone() }
}


pub struct TopicSub<T: Document> {
	topic_id: String,
	subscriber_id: T::Id,
}

impl<T: Document> TopicSub<T> {
	pub fn new(topic: String, subscriber: &T) -> Self {
		Self {
			topic_id: topic,
			subscriber_id: subscriber.id(),
		}
	}

	pub fn topic_id(&self) -> &str { &self.topic_id }
	pub fn subscriber_id(&self) -> &T::Id { &self.subscriber_id }
}


pub trait TopicKey {
	// type Id: DocId;
	fn topic_id(&self) -> Vec<String>;
	fn topic_path(&self) -> String { self.topic_id().join("/") }
}

impl<T: Document> TopicKey for T {
	// type Id = T::Id;
	fn topic_id(&self) -> Vec<String> {
		vec![std::any::type_name::<T>().into(), self.id().into_string()]
	}
}

// ie support tuples of documents..
impl<T1: TopicKey, T2: TopicKey> TopicKey for (&T1, &T2) {
	// type Id = (T1::Id, T2::Id);
	fn topic_id(&self) -> Vec<String> {
		let mut id = Vec::new();
		id.extend(self.0.topic_id());
		id.extend(self.1.topic_id());
		id
	}
}

pub struct TopicId(pub String);


#[cfg(test)]
mod tests {
	use super::*;

	// pub fn new_agent()
	pub struct Message {
		id: PostId,
		/// The actor that created this post.
		author: ActorId,
		topic: String,
		text: String,
	}
	impl Document for Message {
		type Id = PostId;
		fn id(&self) -> Self::Id { self.id }
	}

	#[test]
	fn foobar() {
		let agent = Actor::agent();
		let system = Actor::system();
		let human = Actor::human();

		let system_topic = Topic::<Message>::new_from((&agent, &system));
		let _agent_topic = Topic::<Message>::new_from(agent.clone());
		let actor_topic = Topic::<Message>::new_from(human.clone());


		let display_topic = Topic::<Message>::default();
		TopicSub::new(actor_topic.id(), &display_topic);

		println!("System topic ID: {}", system_topic.id());
	}
}
