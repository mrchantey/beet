lets keep iterating on beet_stack!

## Next Steps -> Router

As a reminder this is an interface agnostic framework, where http requests and ui are represented as the same thing.

Time to refactor Card.
Currently, we have a mishmash route tree, where cards are added directly to the tree, and tools are called as a seperate concept. This has a couple of problems.

1. Stale Data: A card may do some stuff on spawn that is invalid on a second request.
2. Seperate concepts: we have to handle cards as this 'special case' seperate from tools which permeates the crate, see router.rs or route_tree.rs

## Solution



### Interface -> Router

The concept of an interface is nebulous, lets refactor to just use tried and true router concepts.

We should rebrand the `src/interface` module to a router module.

### Cards as tools

A card is actually just a variation of a route_tool, The card() function now accepts the path and a func, probably an IntoToolHandler where Out: Bundle. What happens then needs design work, but my guess is it will shape up similar to `route_tool.rs`, where the outer tool accepts the Request/Response, the inner tool will:



1. get the render() method from the interface
2. 

the card() method will need to: 
1. run the inner tool to get the bundle.
2. spawn the bundle
3. the server should have a child `RouteHidden` render tool, which accepts the Entity and returns a Response. So the cli and repl server might have a markdown renderer whereas the the tui_server might just add CurrentCard to the card and keep it for the next render frame.

```rust

fn card<M,Out:Bundle>(path:&str, handler: impl IntoToolHandler<M>)->impl Bundle{
	(PathPartial::new(path), exchange_tool(handler))
}


struct RenderRequest {
	/// The entity that sent the request, it contains a tool which, when called 
	handler: Entity,
	/// Cards must be run once at first to discover their 
	/// nested tools. In this case discover_call will be true.
	discover_call: bool,
}
struct RenderResponse{
	despawn: bool,
}
```

We should also introduce file_card() which replaces the current `FileContent` paradigm, loading the markdown from disk on each request.

A big change here is that nested tools and cards will not show up in the route tree until they are expanded.


### Caching

This `StatefulInterface` idea may no longer be nessecary. Instead I think we need see `tui_server`, we can just call the root
