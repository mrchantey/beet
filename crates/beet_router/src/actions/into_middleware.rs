use std::path::Path;

use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use bevy::ecs::system::IntoResult;
use bevy::ecs::system::RunSystemError;


pub struct MiddlewareBuilder {
	pub method: Option<HttpMethod>,
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

/// An `action` / `exchange` pair for a current visit.
#[derive(Clone)]
pub struct MiddlewareContext {
	/// The current action this exchange is visiting
	action: Entity,
	/// The `agent` of the action, containing the [`Request`] and [`Response`]
	exchange: Entity,
}

impl MiddlewareContext {
	pub fn action(&self) -> Entity { self.action }
	pub fn exchange(&self) -> Entity { self.exchange }
}

trait MiddlewareIn<M>: Sized {
	fn from_request_meta_and_cx(
		request: &RequestMeta,
		cx: &MiddlewareContext,
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
	fn from_request_meta_and_cx(
		request: &RequestMeta,
		cx: &MiddlewareContext,
	) -> Result<Self, Response> {
		Ok(($($T::from_request_meta_and_cx(request, cx)?,)*))
	}
	}
}
}

variadics_please::all_tuples!(impl_tuple_middleware, 1, 15, T, M);


impl MiddlewareIn<Self> for MiddlewareContext {
	fn from_request_meta_and_cx(
		_: &RequestMeta,
		cx: &MiddlewareContext,
	) -> Result<Self, Response> {
		cx.clone().xok()
	}
}

pub struct FromRequestRefMiddlewareIn;
// includes unit type
impl<T, M> MiddlewareIn<(FromRequestRefMiddlewareIn, M)> for T
where
	T: FromRequestMeta<M>,
{
	fn from_request_meta_and_cx(
		request: &RequestMeta,
		_: &MiddlewareContext,
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
			move |mut ev: On<GetOutcome>,
			      mut commands: Commands,
			      request: Query<&RequestMeta>|
			      -> Result {
				let action = ev.action();
				let exchange = ev.agent();
				let request = request.get(exchange)?;
				let cx = MiddlewareContext { action, exchange };
				let input =
					Input::Inner::from_request_meta_and_cx(request, &cx)?;
				commands.run_system_once_with(
					self.clone().pipe(
						move |result: In<Output>, mut commands: Commands| {
							if let Err(RunSystemError::Failed(err)) =
								result.0.into_result()
							{
								commands
									.entity(exchange)
									.insert(err.into_response());
							}
						},
					),
					input,
				);
				ev.trigger_with_cx(Outcome::Pass);
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
			      request: Query<&RequestMeta>|
			      -> Result {
				let action = ev.action();
				let exchange = ev.agent();
				let request = request.get(exchange)?;
				let cx = MiddlewareContext { action, exchange };
				let this = self.clone();
				let input = Input::from_request_meta_and_cx(request, &cx)?;
				commands.run(async move |world| {
					if let Err(RunSystemError::Failed(err)) =
						this(input.clone(), world.clone()).await.into_result()
					{
						world
							.entity(exchange)
							.insert_then(err.into_response())
							.await;
					}
					world
						.entity(cx.action())
						.trigger_target(Outcome::Pass.with_agent(cx.exchange()))
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
		fn my_system2(_cx: In<MiddlewareContext>) {}
		assert(my_system2);
		assert(|_: In<MiddlewareContext>| {});
		fn my_system3(_cx: In<MiddlewareContext>) -> Result { Ok(()) }
		assert::<(Result, _, _, _, _)>(my_system3);
		// assert(my_system3);
		// assert(|_: In<MiddlewareContext>| -> Result { Ok(()) });
	}

	#[sweet::test]
	async fn async_system() {
		async fn my_async_system(_cx: MiddlewareContext, _world: AsyncWorld) {}
		assert(my_async_system);
		async fn my_async_system2(
			_cx: (MiddlewareContext, MiddlewareContext),
			_world: AsyncWorld,
		) -> Result {
			Ok(())
		}
		assert(my_async_system2);
		assert(async |_cx: MiddlewareContext, _world: AsyncWorld| {});
		assert(
			async |_cx: MiddlewareContext, _world: AsyncWorld| -> Result {
				Ok(())
			},
		);
	}
}
