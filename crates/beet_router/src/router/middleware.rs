use beet_core::prelude::*;
use beet_tool::prelude::*;

/// Declare a tool to be registered as route middleware.
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
		+ IntoTool<T, In = (In, Next<In, Out>), Out = Out>
		+ Default;

impl<T, In, Out> Default for Middleware<T, In, Out>
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
		.get::<Middleware<T, In, Out>>()
		.unwrap()
		.0
		.clone();
	world
		.commands()
		.entity(cx.entity)
		.insert(tool.clone())
		.queue(move |mut entity: EntityWorldMut| {
			entity
				.get_mut_or_default::<MiddlewareList<In, Out>>()
				.0
				.push(tool.into_tool());
		});
}


/// Type-erased collection of middleware tools declared on an ancestor.
///
/// Each entry wraps descendants sharing the same `In`/`Out` signature.
/// Currently used for `Request`/`Response` middleware but generic
/// to support future middleware signatures.
#[derive(Debug, Clone, Component)]
pub struct MiddlewareList<In: 'static, Out: 'static>(
	pub Vec<Tool<(In, Next<In, Out>), Out>>,
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

	pub fn add<T, M>(&mut self, tool: T)
	where
		T: IntoTool<M, In = (In, Next<In, Out>), Out = Out>,
	{
		self.0.push(tool.into_tool());
	}

	/// Apply all middleware in this collection to the given tool,
	/// returning a new tool with each middleware layered on top.
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
