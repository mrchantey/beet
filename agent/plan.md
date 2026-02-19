Lets iterate on beet_stack card.rs.

Whether a card gets spawned is now the responsibility of the render tool, not the card tool.


Below is a rough draft of the behavior
```rust

/// An entity spawned by a [`card`] tool.
#[derive(Debug, Clone, Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = Cards)]
pub struct CardOf(pub Entity);

impl CardOf {
	/// Creates a new attribute relationship pointing to the given entity.
	pub fn new(value: Entity) -> Self { Self(value) }
}

/// All cards spawned by this tool.
#[derive(Debug, Clone, Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = CardOf,linked_spawn)]
pub struct Cards(Vec<Entity>);

#[derive(Debug)]
pub struct RenderRequest {
	/// The entity that made this render request,
	/// which may have existing [`Cards`]
	pub card_tool: Entity,
	/// A tool that spawns the card bundle as an entity,
	/// and adds a [`CardOf(card_tool)`].
	/// The renderer may or may not despawn the returned entity.
	pub spawn_tool: Tool<Request, Entity>,
	/// The original request.
	pub request: Request,
}

/// A card is a tool that creates a [`Bundle`].
/// The spawn/despawn behavior is determined by the nearest [`RenderToolMarker`],
/// discoverd by first walking up the ancestors, and then performing a depth-first search
/// from the root.
/// If the render tool decides to spawn this card, it will be given an [`CardOf`] pointing
/// to this entity.
/// The render tool may also decide to despawn the card.
pub fn card<T, B, M>(path: &str, tool: T) -> impl Bundle
where
	T: IntoTool<M, In = Request, Out = B>,
{
// the signature of the async tool is In=Request, Out=Entity
	func_tool(||{
		// 1. get the render tool
		// 2. call render tool with a RenderRequest
		// 3. Render tool may or may not use the spawn tool, and render the entity
		// 4. Render tool returns a response
	});
	
```

good luck chickadoo


## Entity tool chaining

Chaining tools between entities at runtime.

This is ephemeral, create a tool chain but it isnt actually stored as a component.

I guess its a matter of querying for the render tool


```
query.single.unwrap_err()
renderer not found
```

## Card refactor

remove
- RenderToolMarker
- RenderRequest
