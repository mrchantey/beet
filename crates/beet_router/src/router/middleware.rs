use beet_action::prelude::*;
use beet_core::prelude::*;

/// Declare an action to be registered as route middleware.
/// The component is serializable via reflect and registers
/// itself into [`MiddlewareList`] on add.
#[derive(Debug, Clone, Component)]
#[component(on_add=on_add::<T, In, Out>)]
pub struct Middleware<T, In, Out>(T)
where
	In: 'static,
	Out: 'static,
	T: Component
		+ Clone
		+ IntoAction<T, In = (In, Next<In, Out>), Out = Out>
		+ Default;

impl<T, In, Out> Default for Middleware<T, In, Out>
where
	In: 'static,
	Out: 'static,
	T: Component
		+ Clone
		+ IntoAction<T, In = (In, Next<In, Out>), Out = Out>
		+ Default,
{
	fn default() -> Self { Self(default()) }
}

fn on_add<T, In, Out>(mut world: DeferredWorld, cx: HookContext)
where
	In: 'static,
	Out: 'static,
	T: Component
		+ Clone
		+ IntoAction<T, In = (In, Next<In, Out>), Out = Out>
		+ Default,
{
	let action = world
		.entity(cx.entity)
		.get::<Middleware<T, In, Out>>()
		.unwrap()
		.0
		.clone();
	world
		.commands()
		.entity(cx.entity)
		.insert(action.clone())
		.queue(move |mut entity: EntityWorldMut| {
			entity
				.get_mut_or_default::<MiddlewareList<In, Out>>()
				.0
				.push(action.into_action());
		});
}


/// Type-erased collection of middleware actions declared on an ancestor.
///
/// Each entry wraps descendants sharing the same `In`/`Out` signature.
/// Currently used for `Request`/`Response` middleware but generic
/// to support future middleware signatures.
#[derive(Debug, Clone, Component)]
pub struct MiddlewareList<In: 'static, Out: 'static>(
	pub Vec<Action<(In, Next<In, Out>), Out>>,
);

impl<In, Out> Default for MiddlewareList<In, Out>
where
	In: 'static,
	Out: 'static,
{
	fn default() -> Self { Self(default()) }
}

impl<In, Out> MiddlewareList<In, Out>
where
	In: 'static,
	Out: 'static,
{
	pub fn new() -> Self { Self(vec![]) }

	pub fn add<T, M>(&mut self, action: T)
	where
		T: IntoAction<M, In = (In, Next<In, Out>), Out = Out>,
	{
		self.0.push(action.into_action());
	}

	/// Apply all middleware in this collection to the given action,
	/// returning a new action with each middleware layered on top.
	pub fn wrap(&self, action: &Action<In, Out>) -> Action<In, Out>
	where
		In: 'static + Send + Sync,
		Out: 'static + Send + Sync,
	{
		let mut action = action.clone();
		for wrapper in &self.0 {
			action = wrapper.clone().wrap(action);
		}
		action
	}
}

/// System parameter for resolving ancestor middleware on an entity.
#[derive(SystemParam)]
pub struct MiddlewareQuery<'w, 's, In, Out>
where
	In: 'static,
	Out: 'static,
{
	middleware: AncestorQuery<'w, 's, &'static MiddlewareList<In, Out>>,
}

impl<In, Out> MiddlewareQuery<'_, '_, In, Out>
where
	In: 'static,
	Out: 'static,
{
	/// Wraps an action with all ancestor middleware for the given entity.
	fn resolve_action<M>(
		&self,
		entity: Entity,
		action: impl IntoAction<M, In = In, Out = Out>,
	) -> Action<In, Out>
	where
		In: 'static + Send + Sync,
		Out: 'static + Send + Sync,
	{
		let mut wrapped = action.into_action();
		for list in self.middleware.get_ancestors(entity) {
			wrapped = list.wrap(&wrapped);
		}
		wrapped
	}

	/// Returns `true` if any middleware exists on ancestors
	/// of the given entity.
	#[allow(unused)]
	fn has_middleware(&self, entity: Entity) -> bool {
		let ancestors = self.middleware.get_ancestors(entity);
		for list in ancestors {
			if !list.0.is_empty() {
				return true;
			}
		}
		false
	}
}



#[extend::ext(name=AsyncEntityMiddleware)]
pub impl AsyncEntity {
	fn call_with_middleware<In, Out>(
		&self,
		action: Action<In, Out>,
		input: In,
	) -> MaybeSendBoxedFuture<'_, Result<Out>>
	where
		In: 'static + Send + Sync,
		Out: 'static + Send + Sync,
	{
		Box::pin(async move {
			let action = self
				.with_state::<MiddlewareQuery<In, Out>, _>(
					move |entity, query| query.resolve_action(entity, action),
				)
				.await;
			self.call_detached(action, input).await
		})
	}
}
