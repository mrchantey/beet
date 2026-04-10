use crate::prelude::*;
use beet_core::prelude::*;


/// Declare a tool to be registered as a descendant wrapper,
/// while still being serializable via reflect.
#[derive(Debug, Clone, Component)]
#[component(on_add=on_add::<T, In, Out>)]
pub struct WrapDescendants<T, In, Out>(T)
where
	In: 'static,
	Out: 'static,
	T: Component
		+ Clone
		+ IntoTool<T, In = (In, Next<In, Out>), Out = Out>
		+ Default;

impl<T, In, Out> Default for WrapDescendants<T, In, Out>
where
	In: 'static,
	Out: 'static,
	T: Component
		+ Clone
		+ IntoTool<T, In = (In, Next<In, Out>), Out = Out>
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
		+ IntoTool<T, In = (In, Next<In, Out>), Out = Out>
		+ Default,
{
	let tool = world
		.entity(cx.entity)
		.get::<WrapDescendants<T, In, Out>>()
		.unwrap()
		.0
		.clone();
	world
		.commands()
		.entity(cx.entity)
		.insert(tool.clone())
		.queue(move |mut entity: EntityWorldMut| {
			entity
				.get_mut_or_default::<WrapDescendentsList<In, Out>>()
				.0
				.push(tool.into_tool());
		});
}


/// Type-erased collection of wrapper tools declared on an ancestor.
///
/// Each entry wraps descendants sharing the same `In`/`Out` signature.
#[derive(Debug, Clone, Component)]
pub struct WrapDescendentsList<In: 'static, Out: 'static>(
	pub Vec<Tool<(In, Next<In, Out>), Out>>,
);
impl<In, Out> Default for WrapDescendentsList<In, Out>
where
	In: 'static,
	Out: 'static,
{
	fn default() -> Self { Self(default()) }
}

impl<In, Out> WrapDescendentsList<In, Out>
where
	In: 'static,
	Out: 'static,
{
	pub fn new() -> Self { Self(vec![]) }

	pub fn add<T, M>(&mut self, tool: T)
	where
		T: IntoTool<M, In = (In, Next<In, Out>), Out = Out>,
	{
		self.0.push(tool.into_tool());
	}

	/// Apply all wrappers in this collection to the given tool,
	/// returning a new tool with each wrapper layered on top.
	pub fn wrap(&self, tool: &Tool<In, Out>) -> Tool<In, Out>
	where
		In: 'static + Send + Sync,
		Out: 'static + Send + Sync,
	{
		let mut tool = tool.clone();
		for wrapper in &self.0 {
			tool = wrapper.clone().wrap(tool);
		}
		tool
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[tool(pure)]
	#[derive(Default, Clone, Component)]
	fn Increment(value: u32) -> u32 { value + 1 }

	#[tool]
	#[derive(Default, Clone, Component)]
	async fn Double((value, next): (u32, Next<u32, u32>)) -> Result<u32> {
		let inner = next.call(value).await?;
		Ok(inner * 2)
	}

	#[beet_core::test]
	async fn works() {
		let mut world = AsyncPlugin::world();
		let parent =
			world.spawn(WrapDescendants::<Double, _, _>::default()).id();
		world
			.spawn((ChildOf(parent), Increment))
			.call::<u32, u32>(2)
			.await
			.unwrap()
			// Increments then doubles
			.xpect_eq(6);
	}
}
