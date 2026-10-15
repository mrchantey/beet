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

A request param is an input to whatever runs. An extractor declares a param by taking it. A route's params are the union over everything that runs for it, collected into one `ParamsPattern` that `--help`, the strict refusal and the `400` explanation all read, so none of them can disagree with what the route reads.

An extractor takes its params in its signature. An `#[action]` argument `#[param] fix: bool` takes one param, its doc comment the `--help` description; `entry: Params<EntryParams>` takes every field of a `Reflect` group, a named type existing only when more than one extractor takes it; `any: AnyParams` takes every param the request carries, and `AnyParams::undeclared(&pattern)` is what nothing on the route names. An action taking params and naming no input reads them off the `RequestParts`. The macro turns that one statement into both the read, parsed off the request before the body runs, and the declaration, so nothing is declared in one place and read in another (`beet_action`'s README, "Params: declare by taking", is the macro's side).

A declaration has a scope (`ParamsScope`), decided by the extractor's type. A plain action's scope is the route it sits on, landing its `ParamsDeclaration` in `RouteParams`; middleware and servers scope every route at or beneath them, landing in `SubtreeParams`. The macro infers the scope from a `Next` in the input, so `HelpHandler`'s `#[param] help` reaches every route beneath its router. A hand written extractor declares from its plugin, `app.install(SubtreeParams::declare_on::<Server, T>())`, landing on every entity `Server` is inserted on: a `CliServer` takes `--accept` and `--server`; `HttpServer`, `TuiServer` and `SshTuiServer` take `--server` and the bind knobs `--host --port --ssh-port --path`; whatever themes a page (a `Layout`, `HelpHandler`, a `TuiServer`) takes `--color-scheme` under the name `PageClasses` (`ColorSchemeParams::declare_on`). An extractor built from data pushes from its own hook (`ExchangeGroup`'s `force_param`), and a closure route declares by hand with `RouteParams::new::<T>()`.

The exchange adapter is an extractor too. `route::exchange` and `#[action(route)]` adapt a typed handler by converting the request through `FromRequest` and its output through `IntoResponseWithRequestParts`, and each trait's `params()` declares what its conversion reads, pushed at route reach beside the route's own (`route::exchange_params`, hooked by `route::declare_exchange`). `PageRequest` declares `RenderParams`, so every scene route takes `--root` and `--media-ingest`, and a `Params<T>` input declares its group: one params mechanism, as an argument or as the input.

One collector computes a route's pattern. `ParamsQuery::of(route)` runs per route at every `RouteTree` rebuild, the pattern landing on `ActionNode::params`, and unions three clauses: `SubtreeParams` on the route and every ancestor; `RouteParams` on the route; `RouteParams` on its steps, every descendant that is neither a route (no `PathPartial`) nor beneath one, plus each member of a `Group` a `RunGroup` there runs by reference, wherever it is authored.

```bsx
<CliServer always=true {CallOnReady}>
<Router {HelpHandler}>
	<BuildWasm/>
	<Route path="audit" {ExchangeSequence}>
		<ZoneAudit/>
	</Route>
</Router>
</CliServer>
```

Here `build-wasm` takes its own `--release --features ..`, `audit` its step's `--fix`, and both inherit `--help` and `--color-scheme` from the router and `--accept --server` from the server; neither takes the other's. One key of one type declared twice is two extractors of one param, kept once. One key of two types or arities is a conflict, and a param named after a launch only knob (`--entry`, `--required-features`, ..) never arrives, since `BootstrapConfig::strip` takes those out of the process request before the `CliServer` sees it; either fails the tree build.

Every `CliServer` is strict: it requires `RejectUnknownParams`, with no opt out, because a misspelt flag silently skipped is a different command run. Once routed, a request in a strict url space carrying a param its route's pattern does not hold answers `400` naming each one and the declared param it most plausibly misspells, then pointing at the route's help: ``unknown param `--elevate`, did you mean `--elevated`?``, then `` `deploy --help` lists the params it takes ``. An unmatched path misspelling a route is refused the same way (``unknown route `serv`, did you mean `serve`?``, `RouteTree::closest`), any other left to the not-found answer. A lenient command takes `any: AnyParams` and refuses nothing, ie `run/:route`, which forwards the request to a device's route. The marker governs only the url space it heads, the router it sits on or the one beneath the server it sits on: a nested `Router` is its own url space, so the site a strict CLI serves over http still accepts `?utm_source=..`.

A param the request got wrong, a required one absent or a value its type cannot read (`--port=abc`), is a `ParamError`, the caller's error rather than the server's: a routed one answers `400` naming the param with its type and doc line as `--help` lists it, then pointing at the route's `--help`. A params type the parse cannot read at all, a field with no authored form, is the route author's error and stays a `500`. A CLI exits `64`, a usage error, on any `400`.

`--help` lists the same patterns, so it documents exactly what a strict url space accepts. Each route lists its own params and its steps' beneath it; a middleware declarer at or above the url space root is listed once, under "Inherited params", grouped by extractor; one between the root and a route (a server on a `serve` route) is listed beneath that route; a route taking `AnyParams` is marked as forwarding. The root help adds "Launch", every `BootstrapConfig` knob but the request rank ones (`server`, `path`, `host`, `port`, `ssh-port`), and "Before the build", the registered prescans. The `beet-create-cli` skill walks through building a CLI on all of this.

A command line rebuilt from a request, like the one a refused deploy suggests running with `--elevated`, repeats only `ParamsPattern::recognised`, never the raw argv, which may hold the very typo that led there.

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
