lets keep iterating on beet_stack!

The last iteration was pretty good but we ended up with a bit of confusion.


- card() must accept an IntoToolHandler<Out=B>, not just a func. This is the purpose of RenderRequest::handler, we call that instead of this CardSpawner, remove the CardSpawner, ie file_card should internally just call card()
- find_render_tool, thats utterly unreadable, use run_system_once_with(|In<Entity>, ancestors: Qery<&ChildOf>, children:Query<&Children|{
	let root = ancestors.root(entity);
  children.iter_descendents_inclusive(root).find...()
})
- the render tool is the responsibility of the server not the interface.


```rust
struct RenderRequest {
	/// The handler entity with a tool signature `(Request)/Entity`, where the returned entity is the spawned instance of the card.
	handler: Entity,
	/// Cards must be run once at first to discover their 
	/// nested tools. In this case discover_call will be true.
	/// Use this flag to avoid unnessecary on mount work.
	discover_call: bool,
	/// The original request
	request: Request,
}
```
The reason why we dont just spawn first and pass that to the render tool, is some renderers are stateful like tui, and like if a user revisits the same path we dont need to rebuild the whole card.


A big change here is that nested tools and cards will not show up in the route tree until they are expanded. The route tree will need to first discover all cards, then call each of them, spawning the card, collecting its routes, then despawning it.

This means basically removing all calls to `render_markdown_for`, so that render_markdown becomes one of these 'render tools'
