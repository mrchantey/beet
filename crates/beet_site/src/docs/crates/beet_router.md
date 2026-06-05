+++
title = "beet_router"
+++

# beet_router

`beet_router` is the semantic layer between an application and the interfaces it speaks through. A router is an entity hierarchy: each route is a child carrying a path pattern and an [`Action`]. A request arrives, whatever its transport, and is matched against the tree and dispatched to the action that fits.

Because the routes know nothing about where a request came from, one set of routes serves a command line, an HTTP server and an interactive prompt at the same time. You choose the interface by picking the IO layer (`CliServer`, `HttpServer` or `ReplServer`); the routes do not change.

```rust,ignore
use beet::prelude::*;

fn setup(mut commands: Commands) {
	commands.spawn((
		// pick the IO layer: CliServer, HttpServer or ReplServer
		CliServer::default(),
		// default_router adds route lookup and the built-in app routes
		(default_router(), children![
			exchange_route("", Action::<(), &str>::new_pure(|_| "hello world")),
			exchange_route("about", Action::<(), &str>::new_pure(|_| "about")),
		]),
	));
}
```

Routing is the place where beet's "interface is just IO" philosophy becomes concrete, and it does more than dispatch. It can generate routes from a file tree (`codegen` feature), render route output into [beet_ui](/docs/crates/beet_ui) scene trees, export a static site, and run on `no_std` targets through an embedded route core. The website you are reading is one such router rendered to HTML.
