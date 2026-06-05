# beet_router

An opinionated semantic layer between applications and interfaces.

A router is an entity hierarchy: each route is a child carrying a path pattern and an [`Action`]. Incoming requests, whatever the transport (CLI args, HTTP, a REPL), are matched against the tree and dispatched to the matching action. The same routes therefore serve a command line, an HTTP server and an interactive prompt with no changes.

```rust,ignore
use beet::prelude::*;

fn setup(mut commands: Commands) {
	commands.spawn((
		// pick the IO layer: CliServer, HttpServer or ReplServer
		CliServer::default(),
		// default_router adds route lookup and the built-in app routes,
		// wrapping the user routes declared as children
		(default_router(), children![
			exchange_route("", Action::<(), &str>::new_pure(|_| "hello world")),
			exchange_route("about", Action::<(), &str>::new_pure(|_| "about")),
		]),
	));
}
```

Beyond dispatch it provides route codegen from a file tree (`codegen` feature), rendering route content into [`beet_ui`] scene trees, static site export, and an `embedded` route core for `no_std` targets.
