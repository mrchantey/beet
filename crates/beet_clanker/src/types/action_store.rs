use crate::prelude::*;
use async_lock::RwLock;
use beet_core::prelude::*;
use bevy::tasks::BoxedFuture;
use std::sync::Arc;

use crate::types::ActionId;

#[derive(Clone, Deref, Component)]
pub struct ActionStore(Arc<dyn ActionStoreProvider>);

impl ActionStore {
	pub fn new(provider: impl ActionStoreProvider + 'static) -> Self {
		Self(Arc::new(provider))
	}
	pub fn inner(&self) -> Arc<dyn ActionStoreProvider> { self.0.clone() }
}


impl Default for ActionStore {
	fn default() -> Self { Self::new(MemoryActionStore::default()) }
}

pub trait ActionStoreProvider: 'static + Send + Sync {
	/// Searches the thread for the most recent action with
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
	fn thread_actions(
		&self,
		thread_id: ThreadId,
		after_action: Option<ActionId>,
	) -> BoxedFuture<'_, Result<Vec<Action>>>;
	fn full_thread_actions(
		&self,
		thread_id: ThreadId,
		after_action: Option<ActionId>,
	) -> BoxedFuture<'_, Result<Vec<(Action, Actor)>>>;

	fn insert_actor(&self, actor: Actor) -> BoxedFuture<'_, Result<()>>;
	fn insert_thread(&self, thread: Thread) -> BoxedFuture<'_, Result<()>>;
	fn insert_actions(
		&self,
		actions: Vec<Action>,
	) -> BoxedFuture<'_, Result<()>>;
	fn insert_response_metas(
		&self,
		metas: Vec<ResponseMeta>,
	) -> BoxedFuture<'_, Result<()>>;
}

impl ActionStoreProvider for Arc<dyn ActionStoreProvider> {
	fn stored_response_meta<'a>(
		&'a self,
		provider_slug: &'a str,
		model_slug: &'a str,
		thread_id: ThreadId,
	) -> BoxedFuture<'a, Result<Option<ResponseMeta>>> {
		self.as_ref()
			.stored_response_meta(provider_slug, model_slug, thread_id)
	}

	fn thread_actions(
		&self,
		thread_id: ThreadId,
		after_action: Option<ActionId>,
	) -> BoxedFuture<'_, Result<Vec<Action>>> {
		self.as_ref().thread_actions(thread_id, after_action)
	}

	fn full_thread_actions(
		&self,
		thread_id: ThreadId,
		after_action: Option<ActionId>,
	) -> BoxedFuture<'_, Result<Vec<(Action, Actor)>>> {
		self.as_ref().full_thread_actions(thread_id, after_action)
	}

	fn insert_actor(&self, actor: Actor) -> BoxedFuture<'_, Result<()>> {
		self.as_ref().insert_actor(actor)
	}

	fn insert_thread(&self, thread: Thread) -> BoxedFuture<'_, Result<()>> {
		self.as_ref().insert_thread(thread)
	}
	fn insert_actions(
		&self,
		actions: Vec<Action>,
	) -> BoxedFuture<'_, Result<()>> {
		self.as_ref().insert_actions(actions)
	}

	fn insert_response_metas(
		&self,
		metas: Vec<ResponseMeta>,
	) -> BoxedFuture<'_, Result<()>> {
		self.as_ref().insert_response_metas(metas)
	}
}

/// An in-memory action store
#[derive(Default)]
pub struct MemoryActionStore {
	map: Arc<RwLock<ContextMap>>,
}

impl ActionStoreProvider for MemoryActionStore {
	fn stored_response_meta<'a>(
		&'a self,
		provider_slug: &'a str,
		model_slug: &'a str,
		thread_id: ThreadId,
	) -> BoxedFuture<'a, Result<Option<ResponseMeta>>> {
		Box::pin(async move {
			let map = self.map.read().await;
			map.thread_actions(thread_id, None)
				.into_iter()
				.filter_map(|action| {
					map.metas()
						.values()
						.find(|meta| {
							meta.action_id == action.id()
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
	fn thread_actions(
		&self,
		thread_id: ThreadId,
		after_action: Option<ActionId>,
	) -> BoxedFuture<'_, Result<Vec<Action>>> {
		Box::pin(async move {
			let map = self.map.read().await;

			// 1. get all actions in thread
			let mut actions: Vec<Action> = map
				.actions()
				.values()
				.filter(|action| action.thread() == thread_id)
				.map(|action| action.clone())
				.collect();
			actions.sort();

			// 2. filter by after if provided
			if let Some(after) = after_action {
				match actions.iter().position(|a| a.id() == after) {
					Some(i) => actions[i + 1..].to_vec(),
					None => actions,
				}
			} else {
				actions
			}
			.xok()
		})
	}

	// do not duplicate this technique in sql, use proper queries
	fn full_thread_actions(
		&self,
		thread_id: ThreadId,
		after_action: Option<ActionId>,
	) -> BoxedFuture<'_, Result<Vec<(Action, Actor)>>> {
		Box::pin(async move {
			let map = self.map.read().await;
			let actors = map.actors();
			self.thread_actions(thread_id, after_action)
				.await?
				.into_iter()
				.xtry_map(|action| -> Result<(Action, Actor)> {
					let actor = actors.get(action.author())?.clone();
					Ok((action, actor))
				})?
				.xok()
		})
	}

	fn insert_actor<'a>(&'a self, actor: Actor) -> BoxedFuture<'a, Result<()>> {
		Box::pin(async move {
			let mut map = self.map.write().await;
			map.actors_mut().insert(actor.clone());
			Ok(())
		})
	}

	fn insert_thread<'a>(
		&'a self,
		thread: Thread,
	) -> BoxedFuture<'a, Result<()>> {
		Box::pin(async move {
			let mut map = self.map.write().await;
			map.threads_mut().insert(thread.clone());
			Ok(())
		})
	}

	fn insert_actions(
		&self,
		actions: Vec<Action>,
	) -> BoxedFuture<'_, Result<()>> {
		Box::pin(async move {
			let mut map = self.map.write().await;
			for action in actions {
				map.actions_mut().insert(action);
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
