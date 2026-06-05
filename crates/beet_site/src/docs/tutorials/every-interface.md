+++
title = "Speak every interface"
+++

# Speak every interface

In this tutorial we will write two routes and serve them, first from the command line and then over HTTP, without changing a single route. This is beet's central trick: an interface is just a way in and out, so the same application can answer all of them.

## Set up the project

Create a new binary crate and add beet with the `http_server` feature, which bundles the router and both server backends:

```sh
cargo new hello-routes
cd hello-routes
cargo add beet --features http_server
```

## Write the routes

Open `src/main.rs` and replace its contents with this:

```rust
use beet::prelude::*;

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, ClientAppPlugin))
		.add_systems(Startup, setup)
		.run()
}

fn setup(mut commands: Commands) {
	commands.spawn((
		// the CLI is our way in and out for now
		CliServer::default(),
		(default_router(), children![
			exchange_route("", Action::<(), &str>::new_pure(|_| "hello world")),
			exchange_route("about", Action::<(), &str>::new_pure(|_| "about")),
		]),
	));
}
```

We spawn a `CliServer` next to a router holding two routes. `default_router` adds route lookup and a few built-in routes on top of ours.

## Run it from the command line

```sh
cargo run
```

After the first build you should see the root route's reply:

```text
hello world
```

Now ask for the other route by passing it as an argument:

```sh
cargo run -- about
```

```text
about
```

Notice that the argument became the request path. Try `cargo run -- --help` and you will see both routes listed: the CLI's help is the router's sitemap.

## Now serve the same routes over HTTP

Change one line. Swap `CliServer::default()` for `HttpServer::default()`:

```rust
		// the same routes, now reachable over HTTP
		HttpServer::default(),
```

Run it again. This time the process keeps running, listening on port 8337:

```sh
cargo run
```

In another terminal, ask for each route:

```sh
curl localhost:8337
curl localhost:8337/about
```

```text
hello world
about
```

Notice what did *not* change. The routes are identical; only the server in front of them is different. Stop the server with Ctrl-C when you are done.

## What you have built

You have served one set of routes over two completely different interfaces. The routes never learned where their requests came from, which is exactly the point: in beet, the CLI, an HTTP server and a REPL are interchangeable front doors to the same application. Next, [Your first agent](/docs/tutorials/first-agent) puts an LLM behind one of these doors.
