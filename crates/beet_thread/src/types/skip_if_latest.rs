use crate::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;

#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[require(Tool<(), Outcome> = Self::default_tool())]
pub struct SkipIfLatest<T> {
	#[reflect(ignore)]
	_phantom: PhantomData<T>,
}

impl<T> DefaultTool<(), Outcome> for SkipIfLatest<T> {
	fn default_tool() -> Tool<(), Outcome> {
		async_tool(move |cx: AsyncToolIn| async move {
			let should_skip = cx
				.caller
				.with_state::<ThreadQuery, _>(|entity, query| -> Result<bool> {
					let thread = query.thread(entity)?;
					println!("entity: {}", entity);
					for post in thread.posts() {
						println!(
							"post: {:?} , entity: {}",
							post.actor.kind(),
							post.actor_entity
						);
					}

					if let Some(last) =
						query.thread(entity)?.posts().into_iter().last()
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
				todo!("how to get inner state?");
				// let inner = cx.caller.get_cloned::<Self>().await?.inner;
				// cx.caller.call_detached(inner, ()).await
			}
		})
	}
}

impl<T> SkipIfLatest<T> {
	/// Create a new `SkipIfLatest` wrapper.
	pub fn new() -> Self {
		Self {
			_phantom: default(),
		}
	}
}
