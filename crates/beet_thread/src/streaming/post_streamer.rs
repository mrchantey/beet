use crate::prelude::*;
use beet_core::prelude::*;
use futures::Stream;
use std::borrow::Cow;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDef {
	pub provider_slug: Cow<'static, str>,
	pub model_slug: Cow<'static, str>,
	pub url: Cow<'static, str>,
	pub auth: Option<String>,
}


pub trait PostStreamer {
	fn provider_slug(&self) -> Cow<'static, str>;
	fn model_slug(&self) -> Cow<'static, str>;

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
	provider_slug: Cow<'static, str>,
	model_slug: Cow<'static, str>,
	/// store the response partial with all posts drained,
	/// for using metadata.
	response: Option<ResponsePartial>,
}

impl PostStream {
	pub fn new(
		provider_slug: Cow<'static, str>,
		model_slug: Cow<'static, str>,
		agent: ActorId,
		thread: ThreadId,
		inner: ResPartialStream,
	) -> Self {
		Self {
			provider_slug,
			model_slug,
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
		let provider_slug = self.provider_slug.to_string();
		let model_slug = self.model_slug.to_string();
		MetaBuilder {
			provider_slug,
			model_slug,
			response_id: response.response_id.clone(),
			response_stored: response.response_stored,
		}
		.xok()
	}
}

pub struct MetaBuilder {
	provider_slug: String,
	model_slug: String,
	response_id: String,
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
