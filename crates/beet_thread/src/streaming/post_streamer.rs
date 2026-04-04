use crate::prelude::*;
use beet_core::prelude::*;
use futures::Stream;

use std::pin::Pin;
use std::task::Context;
use std::task::Poll;


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct ModelDef {
	pub provider_slug: SmolStr,
	pub model_slug: SmolStr,
	pub url: SmolStr,
	pub auth: Option<String>,
}


pub trait PostStreamer {
	fn provider_slug(&self) -> &str;
	fn model_slug(&self) -> &str;

	fn stream_posts(
		&mut self,
		caller: AsyncEntity,
	) -> BoxedFuture<'_, Result<PostStream>>;
}


pub type ResPartialStream =
	Pin<Box<dyn Stream<Item = Result<ResponsePartial>> + Send + Sync>>;


/// Processes typed streaming events into [`PostChanges`] values.
pub struct PostStream {
	inner: ResPartialStream,
	agent: ActorId,
	thread: ThreadId,
	// store partial posts as they are built,
	// cloning and returning on each stream part
	posts: TableMap<Post>,
	post_partial_map: PostPartialMap,
	provider_slug: SmolStr,
	model_slug: SmolStr,
	/// store the response partial with all posts drained,
	/// for using metadata.
	response: Option<ResponsePartial>,
}

impl PostStream {
	pub fn new(
		provider_slug: impl Into<SmolStr>,
		model_slug: impl Into<SmolStr>,
		agent: ActorId,
		thread: ThreadId,
		inner: ResPartialStream,
	) -> Self {
		Self {
			provider_slug: provider_slug.into(),
			model_slug: model_slug.into(),
			agent,
			thread,
			inner,
			posts: default(),
			post_partial_map: default(),
			response: None,
		}
	}

	/// Returns an iterator over the collected posts.
	pub fn posts(&self) -> &TableMap<Post> { &self.posts }

	pub fn meta_builder(&self) -> Result<MetaBuilder> {
		let response = self
			.response
			.as_ref()
			.ok_or_else(|| {
				bevyhow!(
					"response id is required to write posts, did the stream finish?"
				)
			})?
			.clone();
		MetaBuilder {
			provider_slug: self.provider_slug.clone(),
			model_slug: self.model_slug.clone(),
			response_id: response.response_id.clone().into(),
			response_stored: response.response_stored,
		}
		.xok()
	}
}

pub struct MetaBuilder {
	provider_slug: SmolStr,
	model_slug: SmolStr,
	response_id: SmolStr,
	response_stored: bool,
}
impl MetaBuilder {
	pub fn build(&self, post_id: PostId) -> ResponseMeta {
		ResponseMeta {
			post_id,
			provider_slug: self.provider_slug.clone(),
			model_slug: self.model_slug.clone(),
			response_id: self.response_id.clone(),
			response_stored: self.response_stored,
		}
	}
}

#[derive(Debug, Default)]
pub struct PostChanges {
	pub created: Vec<Post>,
	pub modified: Vec<Post>,
}

impl PostChanges {
	pub fn is_empty(&self) -> bool {
		self.created.is_empty() && self.modified.is_empty()
	}

	pub fn iter_all(&self) -> impl Iterator<Item = &Post> {
		self.created.iter().chain(self.modified.iter())
	}
}


impl Stream for PostStream {
	type Item = Result<PostChanges>;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		match this.inner.as_mut().poll_next(cx) {
			Poll::Ready(Some(Ok(mut res_partial))) => {
				trace!("Streaming Event: {:#?}", res_partial);
				let post_partials = res_partial.take_posts();
				this.response = res_partial.xsome();

				// get next state without the posts
				let changes = this.post_partial_map.apply_posts(
					&mut this.posts,
					this.agent,
					this.thread,
					post_partials,
				)?;

				Poll::Ready(Some(Ok(changes)))
			}
			Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}
