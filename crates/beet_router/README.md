# `beet_router`

IO agnostic routing utilities.

This crate provides a flexible routing system that uses the ECS pattern for request handling. Routes are defined as control flow hierarchies of middleware, predicates,handlers, and middleware.

# Features

- **ECS-based routing**: Routes are entities with components
- **IO agnostic**: Suitable for http routes, cli commands or clanker tool calls
- **Behavior tree control flow**: Uses `beet_flow` for request routing decisions
- **Composable middleware**: Middleware as components that can be mixed freely
- **Type-safe extractors**: Extract typed data from requests

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
				// swap out the server to route cli subcommands!
				// CliServer::default(),
				HttpServer::new(5000),
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

`axum` is one of the fastest http routers in the world, intended for extreme traffic scenarios like a proxy servers handling 10,000 requests/second.

`beet_router` is designed for application level routing, balancing performance with flexibility. It is also agnostic to the IO, for instance it can also be used for cli subcommands.
