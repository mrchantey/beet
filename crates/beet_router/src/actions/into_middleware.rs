use std::path::Path;

use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use bevy::ecs::system::IntoResult;
use bevy::ecs::system::RunSystemError;


pub struct MiddlewareBuilder {
	// pub method: Option<HttpMethod>,
}

impl MiddlewareBuilder {
	/// Create a new endpoint with the provided [`IntoMiddleware`] handler.
	/// Middleware defaults to accepting a partial path match (allows trailing segments),
	/// and accepting any [`HttpMethod`]
	pub fn new<M>(
		handler: impl 'static + Send + Sync + IntoMiddleware<M>,
	) -> impl Bundle {
		handler.into_middleware()
	}
	/// Create a new endpoint with the provided [`IntoMiddleware`] handler.
	/// Middleware defaults to accepting a partial path match (allows trailing segments),
	/// and accepting any [`HttpMethod`]
	pub fn with_path<M>(
		path: impl AsRef<Path>,
		handler: impl 'static + Send + Sync + IntoMiddleware<M>,
	) -> impl Bundle {
		(
			Name::new("Middleware Selector"),
			Sequence,
			PathPartial::new(path),
			children![partial_path_match(), handler.into_middleware()],
		)
		// .with_handler_bundle(handler.into_middleware())
	}
}

/// Helper for defining methods that do not consume the request, and may
/// or may not insert a response.
/// Note that the [`Observer`] variant must trigger an [`Outcome::Pass`]
/// on the `action` with the `exchange` as its agent.
/// TODO tuples ie [`(MiddlewareContext, RoutePath, QueryParams)`]
pub trait IntoMiddleware<M> {
	fn into_middleware(self) -> impl Bundle;
}

trait MiddlewareIn<M>: Sized {
	fn from_request_meta_and_action(
		request: &RequestMeta,
		action: Entity,
	) -> Result<Self, Response>;
}

pub struct TupleMiddlewareIn;
macro_rules! impl_tuple_middleware {
($(#[$meta:meta])* $(($T:ident, $M:ident)),*) => {
	$(#[$meta])*
	impl<$($T,)* $($M,)*> MiddlewareIn<(TupleMiddlewareIn, $($M,)*)> for ($($T,)*)
	where
		$($T: MiddlewareIn<$M>,)*
	{
	fn from_request_meta_and_action(
		request: &RequestMeta,
		action: Entity,
	) -> Result<Self, Response> {
		Ok(($($T::from_request_meta_and_action(request, action)?,)*))
	}
	}
}
}

variadics_please::all_tuples!(impl_tuple_middleware, 1, 15, T, M);


impl MiddlewareIn<Self> for Entity {
	fn from_request_meta_and_action(
		_: &RequestMeta,
		action: Entity,
	) -> Result<Self, Response> {
		action.xok()
	}
}

pub struct FromRequestRefMiddlewareIn;
// includes unit type
impl<T, M> MiddlewareIn<(FromRequestRefMiddlewareIn, M)> for T
where
	T: FromRequestMeta<M>,
{
	fn from_request_meta_and_action(
		request: &RequestMeta,
		_: Entity,
	) -> Result<Self, Response> {
		T::from_request_meta(request)
	}
}

pub struct SystemIntoMiddleware;
impl<System, Input, Output, M1, M2>
	IntoMiddleware<(Output, Input, SystemIntoMiddleware, M1, M2)> for System
where
	System: 'static + Send + Sync + Clone + IntoSystem<Input, Output, M1>,
	Input: 'static + Send + SystemInput,
	for<'a> Input::Inner<'a>: 'static + Send + Sync + MiddlewareIn<M2>,
	Output: 'static + IntoResult<()>,
	M1: 'static,
{
	fn into_middleware(self) -> impl Bundle {
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      mut commands: Commands,
			      agent_query: AgentQuery<(Entity, &RequestMeta)>|
			      -> Result {
				let action = ev.target();
				let (agent, request) = agent_query.get(action)?;
				let input = Input::Inner::from_request_meta_and_action(
					request, action,
				)?;
				commands.run_system_once_with(
					self.clone().pipe(
						move |result: In<Output>, mut commands: Commands| {
							if let Err(RunSystemError::Failed(err)) =
								result.0.into_result()
							{
								commands
									.entity(agent)
									.insert(err.into_response());
							}
						},
					),
					input,
				);
				commands.entity(action).trigger_target(Outcome::Pass);
				Ok(())
			},
		)
	}
}

pub struct AsyncSystemIntoMiddleware;
impl<Func, Fut, Input, Output, M1>
	IntoMiddleware<(AsyncSystemIntoMiddleware, Input, Output, M1)> for Func
where
	Func: 'static + Send + Sync + Clone + FnOnce(Input, AsyncWorld) -> Fut,
	Fut: Send + Future<Output = Output>,
	Output: 'static + IntoResult<()>,
	Input: 'static + Send + Clone + MiddlewareIn<M1>,
{
	fn into_middleware(self) -> impl Bundle {
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      mut commands: AsyncCommands,
			      agent_query: AgentQuery<(Entity, &RequestMeta)>|
			      -> Result {
				let action = ev.target();
				let (agent, request) = agent_query.get(action)?;
				let this = self.clone();
				let input =
					Input::from_request_meta_and_action(request, action)?;
				commands.run(async move |world| {
					let error_response =
						if let Err(RunSystemError::Failed(err)) =
							this(input.clone(), world.clone())
								.await
								.into_result()
						{
							Some(err.into_response())
						} else {
							None
						};
					// combine insert and trigger into one world access
					// to avoid race with entity despawn after response insert
					world
						.with_then(move |world| {
							if let Some(response) = error_response {
								world.entity_mut(agent).insert(response);
							}
							world
								.entity_mut(action)
								.trigger_target(Outcome::Pass);
						})
						.await;
				});
				Ok(())
			},
		)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn assert<M>(_: impl IntoMiddleware<M>) {}

	#[test]
	fn infers_type() {
		fn my_system() {}
		assert(my_system);
		assert(|| {});
		fn my_system2(_action: In<Entity>) {}
		assert(my_system2);
		assert(|_: In<Entity>| {});
		fn my_system3(_action: In<Entity>) -> Result { Ok(()) }
		assert::<(Result, _, _, _, _)>(my_system3);
		// assert(my_system3);
		// assert(|_: In<MiddlewareContext>| -> Result { Ok(()) });
	}

	#[sweet::test]
	async fn async_system() {
		async fn my_async_system(_action: Entity, _world: AsyncWorld) {}
		assert(my_async_system);
		async fn my_async_system2(
			_action: (Entity, Entity),
			_world: AsyncWorld,
		) -> Result {
			Ok(())
		}
		assert(my_async_system2);
		assert(async |_action: Entity, _world: AsyncWorld| {});
		assert(async |_action: Entity, _world: AsyncWorld| -> Result {
			Ok(())
		});
	}
}
