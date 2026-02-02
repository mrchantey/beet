# `beet_router`

Transport agnostic routing for bevy applications.

This crate provides a flexible routing system that uses the ECS pattern for request handling. Routes are defined as control flow hierarchies of predicates, handlers, and middleware.

The `Request` / `Response` pattern is generalized and not tied to any transport, with implementations in `http` and `stdio`. See [this blog post](https://beetstack.dev/blog/post-8) for more about the agnostic philosophy.

# Features

- **Transport agnostic**: Suitable for http routes, cli commands or clanker tool calls
- **ECS-based routing**: Route trees as entity hierarchies
- **Extensible control flow**: Modular control flow enables highly extensible routing decisions

# Example

```rust,ignore
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			RouterPlugin::default(),
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((
				// swap out the server to route http requests!
				CliServer::default(),
				// HttpServer::default(),
				flow_exchange(|| {
					(InfallibleSequence, children![
						EndpointBuilder::get()
							.with_action(|| { Response::ok_body("hello world", "text/plain")}),
						EndpointBuilder::get()
							.with_path("foo")
							.with_action(|| { Response::ok_body("hello foo", "text/plain")}),
						),
					])
				}),
			));
		})
		.run();
}
```

# Axum Comparison

`axum` is intended for extreme traffic scenarios like a proxy servers handling 10,000 requests/second.

`beet_router` is designed for application level routing, balancing performance with flexibility. It is also agnostic to the IO, for instance it can also be used for cli subcommands.
