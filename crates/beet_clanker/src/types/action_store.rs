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
	fn previous_o11s_meta<'a>(
		&'a self,
		provider_slug: &'a str,
		model_slug: &'a str,
		thread_id: ThreadId,
	) -> BoxedFuture<'a, Result<Option<ActionId>>>;
	fn thread_actions(
		&self,
		thread_id: ThreadId,
		after_action: Option<ActionId>,
	) -> BoxedFuture<'_, Result<Vec<Action>>>;
	fn full_thread_actions(
		&self,
		thread_id: ThreadId,
		after_action: Option<ActionId>,
	) -> BoxedFuture<'_, Result<Vec<(Action, Actor, Option<O11sMeta>)>>>;
}

/// An in-memory action store
pub struct MemoryActionStore {
	map: Arc<RwLock<ContextMap>>,
}

impl ActionStore for MemoryActionStore {
	fn previous_o11s_meta<'a>(
		&'a self,
		provider_slug: &'a str,
		model_slug: &'a str,
		thread_id: ThreadId,
	) -> BoxedFuture<'a, Result<Option<ActionId>>> {
		Box::pin(async move {
			let map = self.map.read().await;
			map.thread_actions(thread_id, None)
				.into_iter()
				.filter_map(|action| {
					map.o11s_metas()
						.values()
						.find(|meta| {
							meta.action_id == action.id()
								&& meta.provider_slug == provider_slug
								&& meta.model_slug == model_slug
						})
						.map(|_meta| action.id())
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
	) -> BoxedFuture<'_, Result<Vec<(Action, Actor, Option<O11sMeta>)>>> {
		Box::pin(async move {
			let map = self.map.read().await;
			let metas = map.o11s_metas();
			let actors = map.actors();
			self.thread_actions(thread_id, after_action)
				.await?
				.into_iter()
				.xtry_map(
					|action| -> Result<(Action, Actor, Option<O11sMeta>)> {
						let actor = actors.get(action.author())?;
						let mut meta = metas.get(action.id()).ok().cloned();

						// hack: for output calls we created, need to get
						// the call_id of the function call item, so use that.
						// this is used in `o11s_mapper`
						if meta.is_none()
							&& let ActionPayload::FunctionCallOutput(output) =
								action.payload()
						{
							meta = metas
								.get(output.function_call_item)
								.ok()
								.cloned();
						}
						Ok((action, actor.clone(), meta))
					},
				)?
				.xok()
		})
	}
}
