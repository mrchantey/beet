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

## Params

A route declares the flags it reads on its own `Reflect` params type, `#[require(ParamsPartial = ParamsPartial::new::<MyParams>())]`, and reads them back in one call, `parts.parse_params::<MyParams>()`, so `--help` documents exactly what the route enforces (the `beet-create-cli` skill walks through it). A route's `ParamsPattern` collects the partials on it and its ancestors, and its `RouteTree` node adds those its steps declare, the descendants that are not routes themselves: `<Route path="audit" {ExchangeSequence}><ZoneAudit/></Route>` takes `<ZoneAudit/>`'s `--fix`.

A param the request gets wrong, a required one absent or a value its type cannot read (`--port=abc`), is a `ParamError`, the caller's error rather than the server's: it answers `400` (a CLI exits `64`, a usage error), and a routed one names the param with its type and doc line as `--help` lists it, then points at the route's `--help`. A params type the parse cannot read at all, a field with no authored form, is the route author's error and stays a `500`.

Middleware and servers read params too, and an entity holds any number of them, so each declares what it reads on its type rather than its entity, as reflect type data: `#[reflect(Component, DeclaresParams)]` beside `impl DeclaresParams for HelpHandler`. `HelpHandler` declares `--help`, `NavigateHandler` `--navigate`, a `Layout` `--color-scheme`, each server its bind knobs, and a `CliServer` its `--accept` plus every launch knob `BootstrapConfig` reads off the same argv (`--entry`, `--stage`, `--server`, ..). `DeclaredParamsQuery` unions every partial and every declaring component along a request's dispatch path into `DeclaredParams`, each param kept beside its declarer, and `ActionNode::declared_params` adds a route's own pattern: the one union `RejectUnknownParams` checks and `--help` lists, a route's own params beneath it and the rest under the type declaring them, those above every listed route once under **Inherited params**.

A server, or a router, opts into refusing what nothing declares with `RejectUnknownParams`, ie `<CliServer always=true {(CallOnReady, RejectUnknownParams)}>`. Once routed, a request carrying an undeclared param answers `400` (a CLI exits `64`, a usage error) naming each one and the declared param it most plausibly misspells, so `deploy --elevate` asks `did you mean --elevated?` rather than running a plain deploy. It is opt-in because a page must tolerate whatever a link carries (`?utm_source=..`), and it governs only the url space it heads: the site an opted-in CLI serves over http is a nested `Router`, a url space of its own, which keeps accepting them. An unmatched path misspelling a route is refused the same way (`serv` asks `did you mean serve?`, `RouteTree::closest`), any other is left to the not-found answer, and a route handing what it does not name to a program it runs declares `ParamsPartial::new::<T>().forwarding()` (`run-wasm`, whose test-runner flags belong to the module). The beet CLI's own entries, `main.bsx` and `site/main.bsx`, opt in, as does every `examples/infra` entry.

A command line rebuilt from a request, like the one a refused deploy suggests running with `--elevated`, repeats only the `DeclaredParams::recognised` params, never the raw argv, which may hold the very typo that led there.

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

`LivePage::prepare` runs the layouts, resolves the `--root` render param (`main` or `content`, one cascade documented on `RenderRoot`, absent for the whole page) and, when the request names a `--media-ingest` policy, the media resolve step, into a `LivePage` a caller renders through any registered target (`beet_ui`'s `RenderTargets`). A request is `LivePage::respond`, one render whose media type `Accept` negotiates, so a direct caller in `LivePage::scoped` naming the same type gets exactly the body a client does, and both release the page whatever the outcome.

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

The scan reads the ROOT only, resolving each declaration against the type registry with the same coercions a spread gets (`created` becomes a `Date`), and hoists every resolved component onto the route entity. Nothing is built, no hook runs and no child is spawned, which is what lets discovery know a page's title, order and slug before anyone visits it. Unsectioned keys declare the document's `FrontmatterType` (`PageMeta` by default, overridable per dir); a `[Section]` header names its component by short type path.

`PageMeta` is a consumer of that set like any other: the router reads `slug` for the url, `order`/`sidebar_label`/`expanded` for the nav, `visibility` for static export and dispatch, and `<ArticleHeader/>` renders its `title`/`authors`/`created`/`video_url` as the article chrome.

`visibility` is the one knob every listing reads: `Public` serves and is listed everywhere, `Unlisted` serves to whoever holds the link and appears in no index (and is marked `noindex` for crawlers), and `Draft` never reaches production at all — dropped from a prod static export and answered `404` by a prod dispatch, exactly as an unknown url is.

## Syndication

`<Sitemap/>`, `<Robots/>`, `<RssFeed/>` and `<SearchIndex/>` are the machine-readable faces of a site: opt-in tags spawning ordinary static `GET` routes (`sitemap.xml`, `robots.txt`, `rss.xml`, `search-index.json`) that walk the `RouteTree` and serialize it, so they serve live and static-export as files with no special casing.

Position is the whole configuration surface. A `Router` is a url space and routes root at their ancestors, so `<Sitemap/>` under the router root covers the site while `<RssFeed/>` inside `<Route path="blog">` serves at `/blog/rss.xml` and feeds only the posts beneath it. The feed and the index carry each page's real body, its `--root=content` render (html for the feed, plain text for the index), so neither carries the chrome a layout adds.

They need an origin to resolve their absolute urls against, so a `PackageConfig` with no `homepage` fails their dispatch naming the field.

The same listed pages are the source of a site's [standard.site](https://standard.site) records, built with `dag_cbor`: `StandardSitePublication::from_declaration` turns a `StandardSitePub` declaration into the `site.standard.publication` record, and `StandardSiteDocument::render` turns a page's metadata and its live page into its `site.standard.document`, the post as plain text and as Leaflet blocks through the `LeafletRenderer` target, its images uploaded off the same tree. Each record type is exactly its lexicon; the cheatsheet is the module docs of `src/router/site/syndication/standard_site/`.
