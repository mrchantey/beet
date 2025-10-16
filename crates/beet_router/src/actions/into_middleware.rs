use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::IntoResponse;
use bevy::ecs::system::IntoResult;
use bevy::ecs::system::RunSystemError;

/// Helper for defining methods that do not consume the request, and may
/// or may not insert a response.
/// Note that the [`Observer`] variant must trigger an [`Outcome::Pass`]
/// on the `action` with the `exchange` as its agent.
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

trait MiddlewareIn {
	fn from_cx(cx: MiddlewareContext) -> Self;
}
impl MiddlewareIn for MiddlewareContext {
	fn from_cx(cx: MiddlewareContext) -> Self { cx }
}
impl MiddlewareIn for () {
	fn from_cx(_cx: MiddlewareContext) -> Self {}
}

pub struct SystemIntoMiddleware;
impl<System, Input, Output, M1>
	IntoMiddleware<(Output, Input, SystemIntoMiddleware, M1)> for System
where
	System: 'static + Send + Sync + Clone + IntoSystem<Input, Output, M1>,
	Input: 'static + Send + SystemInput,
	for<'a> Input::Inner<'a>: 'static + Send + Sync + MiddlewareIn,
	Output: 'static + IntoResult<()>,
	M1: 'static,
{
	fn into_middleware(self) -> impl Bundle {
		OnSpawn::observe(
			move |mut ev: On<GetOutcome>, mut commands: Commands| {
				let action = ev.action();
				let exchange = ev.agent();
				let cx = MiddlewareContext { action, exchange };
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
					Input::Inner::from_cx(cx),
				);
				ev.trigger_next(Outcome::Pass);
			},
		)
	}
}

pub struct AsyncSystemIntoMiddleware;
impl<Func, Fut, Output> IntoMiddleware<(AsyncSystemIntoMiddleware, Output)>
	for Func
where
	Func: 'static
		+ Send
		+ Sync
		+ Clone
		+ FnOnce(MiddlewareContext, AsyncWorld) -> Fut,
	Fut: Send + Future<Output = Output>,
	Output: 'static + IntoResult<()>,
{
	fn into_middleware(self) -> impl Bundle {
		OnSpawn::observe(
			move |ev: On<GetOutcome>, mut commands: AsyncCommands| {
				let action = ev.action();
				let exchange = ev.agent();
				let cx = MiddlewareContext { action, exchange };
				let this = self.clone();
				commands.run(async move |world| {
					if let Err(RunSystemError::Failed(err)) =
						this(cx.clone(), world.clone()).await.into_result()
					{
						world
							.entity(exchange)
							.insert(err.into_response())
							.await;
					}
					world
						.entity(cx.action())
						.trigger_target(Outcome::Pass.with_agent(cx.exchange()))
						.await;
				});
			},
		)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn assert<M>(_: impl IntoMiddleware<M>) {}

	#[sweet::test]
	async fn system() {
		fn my_system() {}
		assert(my_system);
		assert(|| {});
		fn my_system2(_cx: In<MiddlewareContext>) {}
		assert(my_system2);
		assert(|_: In<MiddlewareContext>| {});
		fn my_system3(_cx: In<MiddlewareContext>) -> Result { Ok(()) }
		assert::<(Result, _, _, _)>(my_system3);
		// assert(my_system3);
		// assert(|_: In<MiddlewareContext>| -> Result { Ok(()) });
	}

	#[sweet::test]
	async fn async_system() {
		async fn my_async_system(_cx: MiddlewareContext, _world: AsyncWorld) {}
		assert(my_async_system);
		async fn my_async_system2(
			_cx: MiddlewareContext,
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
