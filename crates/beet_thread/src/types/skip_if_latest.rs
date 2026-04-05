use crate::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;
use std::marker::PhantomData;

/// Trait for types whose tool handler can be constructed statically.
///
/// This allows [`SkipIfLatest<T>`] to create the inner tool at
/// `#[require]` resolution time without needing an instance of `T`.
pub trait InnerTool: 'static + Send + Sync {
	/// Create the tool handler for this type.
	fn inner_tool() -> Tool<(), Outcome>;
}

impl InnerTool for StdinPost {
	fn inner_tool() -> Tool<(), Outcome> { StdinPost.into_tool() }
}

impl InnerTool for O11sStreamer {
	fn inner_tool() -> Tool<(), Outcome> {
		async_tool(post_streamer_tool::<O11sStreamer>)
	}
}

impl InnerTool for CompletionsStreamer {
	fn inner_tool() -> Tool<(), Outcome> {
		async_tool(post_streamer_tool::<CompletionsStreamer>)
	}
}

/// Wrapper that skips execution if this entity was the latest poster
/// in the thread.
///
/// Replaces the inner tool's [`Tool<(), Outcome>`] via `#[require]`,
/// providing a wrapped handler that checks whether the entity was
/// the most recent poster. If so it returns [`PASS`] without
/// invoking the inner tool; otherwise it delegates to the inner
/// handler created by [`InnerTool::inner_tool`].
///
/// For data-carrying tool components like [`O11sStreamer`], spawn
/// the data component alongside this wrapper — its `on_add` hook
/// will detect the existing tool and skip overwriting it.
///
/// ```rust,no_run
/// # use beet::prelude::*;
/// // Unit-struct tool — SkipIfLatest replaces StdinPost entirely:
/// # fn a(mut commands: Commands) {
/// commands.spawn((
///     Actor::new("User", ActorKind::User),
///     SkipIfLatest::<StdinPost>::new(),
/// ));
/// # }
///
/// // Data tool — keep the component for its config:
/// # fn b(mut commands: Commands) {
/// commands.spawn((
///     Actor::new("Agent", ActorKind::Agent),
///     SkipIfLatest::<O11sStreamer>::new(),
///     OpenAiProvider::gpt_5_mini().unwrap(),
/// ));
/// # }
/// ```
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Tool<(), Outcome> = SkipIfLatest::<T>::make_tool())]
pub struct SkipIfLatest<T: InnerTool = StdinPost>(
	#[reflect(ignore)] PhantomData<fn() -> T>,
);

impl<T: InnerTool> Default for SkipIfLatest<T> {
	fn default() -> Self { Self(PhantomData) }
}

impl<T: InnerTool> SkipIfLatest<T> {
	/// Create a new `SkipIfLatest` wrapper.
	pub fn new() -> Self { Self::default() }

	/// Build the wrapped [`Tool`] used by `#[require]`.
	fn make_tool() -> Tool<(), Outcome> {
		let inner = T::inner_tool();
		async_tool(move |cx: AsyncToolIn| {
			let inner = inner.clone();
			async move {
				let should_skip = cx
					.caller
					.with_state::<ThreadQuery, _>(
						|entity, query| -> Result<bool> {
							if let Some(last) =
								query.thread(entity)?.posts().into_iter().last()
								&& last.actor_entity == entity
							{
								true
							} else {
								false
							}
							.xok()
						},
					)
					.await?;

				if should_skip {
					Ok(PASS)
				} else {
					cx.caller.call_detached(inner, ()).await
				}
			}
		})
	}
}
