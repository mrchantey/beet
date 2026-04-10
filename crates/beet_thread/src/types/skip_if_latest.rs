use crate::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;

#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
#[require(Tool<(), Outcome> = Self::default_tool())]
pub struct SkipIfLatest<T, M = T>
where
	T: 'static + Send + Sync + Clone + IntoTool<M, In = (), Out = Outcome>,
	M: 'static + Send + Sync,
{
	inner: T,
	#[reflect(ignore)]
	_phantom: PhantomData<M>,
}

impl<T, M> Clone for SkipIfLatest<T, M>
where
	T: 'static + Send + Sync + Clone + IntoTool<M, In = (), Out = Outcome>,
	M: 'static + Send + Sync,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			_phantom: default(),
		}
	}
}

impl<T, M> SkipIfLatest<T, M>
where
	T: 'static + Send + Sync + Clone + IntoTool<M, In = (), Out = Outcome>,
	M: 'static + Send + Sync,
{
	/// Create a new `SkipIfLatest` wrapper.
	pub fn new(inner: T) -> Self {
		Self {
			inner,
			_phantom: default(),
		}
	}
}


impl<T, M> DefaultTool<(), Outcome> for SkipIfLatest<T, M>
where
	T: 'static + Send + Sync + Clone + IntoTool<M, In = (), Out = Outcome>,
	M: 'static + Send + Sync,
{
	fn default_tool() -> Tool<(), Outcome> {
		async_tool(move |cx: ToolContext| async move {
			let should_skip = cx
				.caller
				.with_state::<ThreadQuery, _>(|entity, query| -> Result<bool> {
					let thread = query.thread(entity)?;

					if let Some(last) = thread.posts().into_iter().last()
						&& last.actor_entity == entity
					{
						true
					} else {
						false
					}
					.xok()
				})
				.await?;

			if should_skip {
				Ok(PASS)
			} else {
				let inner = cx.caller.get_cloned::<Self>().await?.inner;
				cx.caller.call_detached(inner, ()).await
			}
		})
	}
}
