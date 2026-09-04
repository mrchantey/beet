# beet_router

An opinionated semantic layer between applications and interfaces.

A router is an entity hierarchy: each route is a child carrying a path pattern and an [`Action`]. Incoming requests, whatever the transport (CLI args, HTTP, a REPL), are matched against the tree and dispatched to the matching action. The same routes therefore serve a command line, an HTTP server and an interactive prompt with no changes.

```rust,ignore
use beet::prelude::*;

fn setup(mut commands: Commands) {
	commands.spawn((
		// declare the IO layers; `--server` picks which of them act
		(CliServer::default(), ReplServer {
			default_boot: false,
			..default()
		}),
		CallOnReady::on_spawn(),
		// the dispatch host is a child: route lookup plus the built-in app
		// routes, wrapping the user routes declared as its own children
		children![(Router::with_defaults(), children![
			route::exchange("", Action::<(), &str>::new_pure(|_| "hello world")),
			route::exchange("about", Action::<(), &str>::new_pure(|_| "about")),
		])],
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
            content entity  ◄── cx.content() │  (carries PageMeta, etc)
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
   ├─ head … <title>{ @entity:PageRoot::PageMeta.title }</title>   ◄── RouteHead
   └─ body
       ├─ slot child  [Portal ──► content]    (content spliced in here at render)
       └─ RouteSidebar
```

The matched `route` entity is reachable from **no** layout edge (`Portal` and `LayoutContent` both point at `content`), so the per-request facts a layout widget needs are threaded explicitly through `RequestContext` rather than re-derived by traversal:

- `route` — the matched route entity, the in-tree anchor.
- `router` — the entity that owns this request's `RouteTree`; widgets read the tree off it directly (no ancestor walk).
- `content` — the rendered content entity, off which per-route components (eg `PageMeta`) are queried.

`PageRoot` names the entity the serializer walks (self-referential for a plain route, the layout itself for a wrapped one). `DespawnAfterRender` lists the ephemerals torn down after each render; nothing is cached between requests.

### Layouts

A layout is render middleware, `Layout{template:"Layout"}` declared on an ancestor of the routes it wraps (typically the router). The name resolves exactly as it would in tag position: a `.bsx` document in the `BsxTemplateRegistry` first, else a rust `#[template]` registered by short type path, so markup and rust layouts are one mechanism with one declaration. From rust that name comes from the type, `Layout::of::<SiteLayout>()`, so a rename follows the symbol; `Layout::new("..")` is for a `.bsx` document, which has no type to name.

Layouts **nest**: every ancestor declaring one wraps the route exactly once, furthest ancestor outermost. So a site shell on the router and an article shell on `<Route path="blog">` render as `Layout(ArticleLayout(post))`, and the inner one is chrome only, never a second document.

```bsx
<Router {Layout{template:"Layout"}}>
	<RoutesDir src="routes" filter={GlobFilter{exclude:["blog/**"]}}/>
	<Route path="blog" {Layout{template:"ArticleLayout"}}>
		<RoutesDir src="routes/blog"/>
	</Route>
</Router>
```

Each wrap adds one layout root to the chain, so `LayoutContent` on an outer layout would name the layout beneath it rather than the page. It never does: the wrap resolves `LayoutContent::terminal` first, so both the link and `cx.content()` name the route content however deep the nesting, and a head `<title>` binding reads the post's meta through both shells.

### Page metadata

A document's metadata is **whatever components it declares at its root**. A markdown page writes frontmatter, a BSX page writes bare spreads on its root element, and both lower to the same `RootDeclarations` literals:

```md
+++
title = "Full Stack Bevy"
slug = "full-stack-bevy"
created = "2025-07-11"
[Layout]
template = "ArticleLayout"
+++
```

The scan reads the ROOT only, resolving each declaration against the type registry with the same coercions a spread gets (`created` becomes a `Timestamp`), and hoists every resolved component onto the route entity. Nothing is built, no hook runs and no child is spawned, which is what lets discovery know a page's title, order and slug before anyone visits it. Unsectioned keys declare the document's `FrontmatterType` (`PageMeta` by default, overridable per dir); a `[Section]` header names its component by short type path.

`PageMeta` is a consumer of that set like any other: the router reads `slug` for the url, `order`/`sidebar_label`/`expanded` for the nav, `draft` for static export, and `<ArticleHeader/>` renders its `title`/`author`/`created`/`video_url` as the article chrome.
