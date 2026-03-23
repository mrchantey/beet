use crate::prelude::*;
use async_lock::RwLock;
use beet_core::prelude::*;
use std::sync::Arc;

#[derive(Clone, Deref, Component)]
pub struct PostStore(Arc<dyn PostStoreProvider>);

impl PostStore {
	pub fn new(provider: impl PostStoreProvider + 'static) -> Self {
		Self(Arc::new(provider))
	}
	pub fn inner(&self) -> Arc<dyn PostStoreProvider> { self.0.clone() }
}


impl Default for PostStore {
	fn default() -> Self { Self::new(MemoryPostStore::default()) }
}

pub trait PostStoreProvider: 'static + Send + Sync {
	// fn users(&self) -> &DocMap<User>;
	// fn threads(&self) -> &DocMap<User>;
	// fn posts(&self) -> &DocMap<User>;

	/// Searches the thread for the most recent post with
	/// a [`O11sMeta`] that was stored by the provider,
	/// for use with `previous_response_id` patterns.
	///
	/// The provider and model slugs are also checked to ensure
	/// we get the most recent meta *for this match*, supporting
	/// multi-agent and model-switching scenarios.
	fn stored_response_meta<'a>(
		&'a self,
		provider_slug: &'a str,
		model_slug: &'a str,
		thread_id: ThreadId,
	) -> BoxedFuture<'a, Result<Option<ResponseMeta>>>;
	fn thread_posts(
		&self,
		thread_id: ThreadId,
		after_post: Option<PostId>,
	) -> BoxedFuture<'_, Result<Vec<Post>>>;
	fn full_thread_posts(
		&self,
		thread_id: ThreadId,
		after_post: Option<PostId>,
	) -> BoxedFuture<'_, Result<Vec<(Post, User)>>>;

	fn insert_user(&self, user: User) -> BoxedFuture<'_, Result<UserId>>;
	fn insert_thread(
		&self,
		thread: Thread,
	) -> BoxedFuture<'_, Result<ThreadId>>;
	fn insert_posts(&self, posts: Vec<Post>) -> BoxedFuture<'_, Result<()>>;
	fn insert_response_metas(
		&self,
		metas: Vec<ResponseMeta>,
	) -> BoxedFuture<'_, Result<()>>;
}

impl PostStoreProvider for Arc<dyn PostStoreProvider> {
	fn stored_response_meta<'a>(
		&'a self,
		provider_slug: &'a str,
		model_slug: &'a str,
		thread_id: ThreadId,
	) -> BoxedFuture<'a, Result<Option<ResponseMeta>>> {
		self.as_ref()
			.stored_response_meta(provider_slug, model_slug, thread_id)
	}

	fn thread_posts(
		&self,
		thread_id: ThreadId,
		after_post: Option<PostId>,
	) -> BoxedFuture<'_, Result<Vec<Post>>> {
		self.as_ref().thread_posts(thread_id, after_post)
	}

	fn full_thread_posts(
		&self,
		thread_id: ThreadId,
		after_post: Option<PostId>,
	) -> BoxedFuture<'_, Result<Vec<(Post, User)>>> {
		self.as_ref().full_thread_posts(thread_id, after_post)
	}

	fn insert_user(&self, user: User) -> BoxedFuture<'_, Result<UserId>> {
		self.as_ref().insert_user(user)
	}

	fn insert_thread(
		&self,
		thread: Thread,
	) -> BoxedFuture<'_, Result<ThreadId>> {
		self.as_ref().insert_thread(thread)
	}
	fn insert_posts(&self, posts: Vec<Post>) -> BoxedFuture<'_, Result<()>> {
		self.as_ref().insert_posts(posts)
	}

	fn insert_response_metas(
		&self,
		metas: Vec<ResponseMeta>,
	) -> BoxedFuture<'_, Result<()>> {
		self.as_ref().insert_response_metas(metas)
	}
}

/// An in-memory post store
#[derive(Default)]
pub struct MemoryPostStore {
	map: Arc<RwLock<ContextMap>>,
}


/// An in-memory unindexed table store for short-lived queries.
/// Correctness is prioritized over efficiency, ie no indexes are
/// maintained, and posts are sorted per each 'get'.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ContextMap {
	users: DocMap<User>,
	posts: DocMap<Post>,
	threads: DocMap<Thread>,
	response_metas: DocMap<ResponseMeta>,
}


impl ContextMap {
	pub fn users(&self) -> &DocMap<User> { &self.users }
	pub fn users_mut(&mut self) -> &mut DocMap<User> { &mut self.users }

	pub fn posts(&self) -> &DocMap<Post> { &self.posts }
	pub fn posts_mut(&mut self) -> &mut DocMap<Post> { &mut self.posts }

	// pub fn threads(&self) -> &DocMap<Thread> { &self.threads }
	pub fn threads_mut(&mut self) -> &mut DocMap<Thread> { &mut self.threads }
	pub fn metas(&self) -> &DocMap<ResponseMeta> { &self.response_metas }
	pub fn metas_mut(&mut self) -> &mut DocMap<ResponseMeta> {
		&mut self.response_metas
	}

	/// Returns all posts belonging to the given thread, sorted chronologically.
	pub fn thread_posts(
		&self,
		thread_id: ThreadId,
		posts_after: Option<PostId>,
	) -> Vec<&Post> {
		let mut posts: Vec<&Post> = self
			.posts
			.values()
			.filter(|post| post.thread() == thread_id)
			.collect();
		posts.sort_by_key(|post| post.id());

		if let Some(after) = posts_after {
			let pos = posts
				.iter()
				.position(|post| post.id() == after)
				.map(|i| i + 1)
				.unwrap_or(0);
			posts[pos..].to_vec()
		} else {
			posts
		}
	}
}


impl PostStoreProvider for MemoryPostStore {
	fn stored_response_meta<'a>(
		&'a self,
		provider_slug: &'a str,
		model_slug: &'a str,
		thread_id: ThreadId,
	) -> BoxedFuture<'a, Result<Option<ResponseMeta>>> {
		Box::pin(async move {
			let map = self.map.read().await;
			map.thread_posts(thread_id, None)
				.into_iter()
				.filter_map(|post| {
					map.metas()
						.values()
						.find(|meta| {
							meta.post_id == post.id()
								&& meta.provider_slug == provider_slug
								&& meta.model_slug == model_slug
								// even if its a match, ignore if no response stored
								&& meta.response_stored
						})
						.cloned()
				})
				.last()
				.xok()
		})
	}
	fn thread_posts(
		&self,
		thread_id: ThreadId,
		after_post: Option<PostId>,
	) -> BoxedFuture<'_, Result<Vec<Post>>> {
		Box::pin(async move {
			let map = self.map.read().await;

			// 1. get all posts in thread
			let mut posts: Vec<Post> = map
				.posts()
				.values()
				.filter(|post| post.thread() == thread_id)
				.map(|post| post.clone())
				.collect();
			posts.sort();

			// 2. filter by after if provided
			if let Some(after) = after_post {
				match posts.iter().position(|post| post.id() == after) {
					Some(i) => posts[i + 1..].to_vec(),
					None => posts,
				}
			} else {
				posts
			}
			.xok()
		})
	}

	// do not duplicate this technique in sql, use proper queries
	fn full_thread_posts(
		&self,
		thread_id: ThreadId,
		after_post: Option<PostId>,
	) -> BoxedFuture<'_, Result<Vec<(Post, User)>>> {
		Box::pin(async move {
			let map = self.map.read().await;
			let users = map.users();
			self.thread_posts(thread_id, after_post)
				.await?
				.into_iter()
				.xtry_map(|post| -> Result<(Post, User)> {
					let user = users.get(post.author())?.clone();
					Ok((post, user))
				})?
				.xok()
		})
	}

	fn insert_user(&self, user: User) -> BoxedFuture<'_, Result<UserId>> {
		Box::pin(async move {
			let id = user.id();
			let mut map = self.map.write().await;
			map.users_mut().insert(user.clone());
			Ok(id)
		})
	}

	fn insert_thread(
		&self,
		thread: Thread,
	) -> BoxedFuture<'_, Result<ThreadId>> {
		Box::pin(async move {
			let id = thread.id();
			let mut map = self.map.write().await;
			map.threads_mut().insert(thread.clone());
			Ok(id)
		})
	}

	fn insert_posts(&self, posts: Vec<Post>) -> BoxedFuture<'_, Result<()>> {
		Box::pin(async move {
			let mut map = self.map.write().await;
			for post in posts {
				map.posts_mut().insert(post);
			}
			Ok(())
		})
	}

	fn insert_response_metas(
		&self,
		metas: Vec<ResponseMeta>,
	) -> BoxedFuture<'_, Result<()>> {
		Box::pin(async move {
			let mut map = self.map.write().await;
			for meta in metas {
				map.metas_mut().insert(meta);
			}
			Ok(())
		})
	}
}
