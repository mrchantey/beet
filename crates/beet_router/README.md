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

## Render geometry

Rendering deliberately severs the `ChildOf` hierarchy: the document layout is built **detached** and **ephemeral** (rebuilt per request), and the route's content is spliced into the layout's `<Slot>` *by reference* (a `Portal`), never reparented. So a widget living in the layout (the sidebar, the head) has no parent path back to the route, its tree, or its content. The two trees are connected only by the explicit edges below.

```text
ROUTE TREE  (persistent; RouteTree component lives on the root ancestor)
  router root ───────────────────────────────┐  [RouteTree]  ◄── cx.router()
   └─ … ─ route entity  ◄── cx.route()       │  (the matched action)
                │                            │
                │ for fixed/markdown routes, │
                │ content == route entity;   │
                │ for pure/func routes,      │
                │ content is a DETACHED root │
                ▼                            │
            content entity  ◄── cx.content() │  (carries ArticleMeta, etc)
                ▲     ▲                       
                │     │                       
   ┌────────────┘     └────────────────┐      
   │ Portal (beet_ui)                  │ LayoutContent (beet_core)
   │ slot child → content;             │ layout root → content;
   │ walker splices content in,        │ the head title binding
   │ style cascade inherits through    │ hops it to read the route's meta
   │                                   │
LAYOUT TREE  (ephemeral, detached, rebuilt per request)
  layout root  [PageRoot = self, LayoutContent ──► content]
   ├─ head … <title>{ @entity:PageRoot::ArticleMeta.title }</title>   ◄── RouteHead
   └─ body
       ├─ slot child  [Portal ──► content]    (content spliced in here at render)
       └─ RouteSidebar
```

The matched `route` entity is reachable from **no** layout edge (`Portal` and `LayoutContent` both point at `content`), so the per-request facts a layout widget needs are threaded explicitly through `RequestContext` rather than re-derived by traversal:

- `route` — the matched route entity, the in-tree anchor.
- `router` — the entity that owns this request's `RouteTree`; widgets read the tree off it directly (no ancestor walk).
- `content` — the rendered content entity, off which per-route components (eg `ArticleMeta`) are queried.

`PageRoot` names the entity the serializer walks (self-referential for a plain route, the layout itself for a wrapped one). `DespawnAfterRender` lists the ephemerals torn down after each render; nothing is cached between requests.
