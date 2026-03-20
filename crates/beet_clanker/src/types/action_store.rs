use crate::prelude::*;
use async_lock::RwLock;
use beet_core::prelude::*;
use bevy::tasks::BoxedFuture;
use std::sync::Arc;

use crate::types::ActionId;

pub trait ActionStore: Send + Sync {
	/// Searches the thread for the most recent action with
	/// a [`O11sMeta`], which will contain useful information
	/// for use in types like `previous_response_id`.
	///
	/// The provider and model slugs are also checked to ensure
	/// we get the most recent meta *for this match*, supporting
	/// multi-agent and model-switching scenarios.
	fn previous_meta<'a>(
		&'a self,
		provider_slug: &'a str,
		model_slug: &'a str,
		thread_id: ThreadId,
	) -> BoxedFuture<'a, Result<Option<ActionMeta>>>;
	fn thread_actions(
		&self,
		thread_id: ThreadId,
		after_action: Option<ActionId>,
	) -> BoxedFuture<'_, Result<Vec<Action>>>;
	fn full_thread_actions(
		&self,
		thread_id: ThreadId,
		after_action: Option<ActionId>,
	) -> BoxedFuture<'_, Result<Vec<(Action, Actor, ActionMeta)>>>;
}

impl ActionStore for Arc<dyn ActionStore> {
	fn previous_meta<'a>(
		&'a self,
		provider_slug: &'a str,
		model_slug: &'a str,
		thread_id: ThreadId,
	) -> BoxedFuture<'a, Result<Option<ActionMeta>>> {
		self.as_ref()
			.previous_meta(provider_slug, model_slug, thread_id)
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
	) -> BoxedFuture<'_, Result<Vec<(Action, Actor, ActionMeta)>>> {
		self.as_ref().full_thread_actions(thread_id, after_action)
	}
}

/// An in-memory action store
pub struct MemoryActionStore {
	map: Arc<RwLock<ContextMap>>,
}

impl ActionStore for MemoryActionStore {
	fn previous_meta<'a>(
		&'a self,
		provider_slug: &'a str,
		model_slug: &'a str,
		thread_id: ThreadId,
	) -> BoxedFuture<'a, Result<Option<ActionMeta>>> {
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
	) -> BoxedFuture<'_, Result<Vec<(Action, Actor, ActionMeta)>>> {
		Box::pin(async move {
			let map = self.map.read().await;
			let metas = map.metas();
			let actors = map.actors();
			self.thread_actions(thread_id, after_action)
				.await?
				.into_iter()
				.xtry_map(|action| -> Result<(Action, Actor, ActionMeta)> {
					let actor = actors.get(action.author())?.clone();
					let meta = metas.get(action.id())?.clone();
					Ok((action, actor, meta))
				})?
				.xok()
		})
	}
}
