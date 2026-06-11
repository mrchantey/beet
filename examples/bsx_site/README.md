# BSX Site

A whole site declared in markup: `main.bsx` is the entrypoint, `routes/` is the content, `templates/` holds the site's own BSX templates. No codegen and no Rust authoring, analogous to mdbook but built on the beet router.

```sh
alias site='cargo run --example bsx_site --features "http_server,markdown,style,template,fs" --'

# CLI mode: render the home route, or a named route
site
site docs/getting-started

# HTTP mode
site --server=http

# static export to examples/bsx_site/dist
site export
```

Because everything site-shaped is a runtime file, edits to `main.bsx`, the templates, and the routes need no rebuild, just rerun (or re-request, in HTTP mode).

## How it works

`main.rs` is a thin generic host (the shape a future `beet run main.bsx` command would take): it adds the standard server plugins, registers the widget and site templates, then spawns `main.bsx` as the root entity and layers the host concerns (server backend, dev `export` route) onto it. Everything site-shaped lives in the markup:

```html
<Router {(RequestLogger, HelpHandler, NavigateHandler, BsxLayout{template:"Layout"})}>
	<RoutesDir src="routes"/>
</Router>
```

- `<Router>` is the `beet_router` dispatch component, resolved by reflection. `main.bsx` has a single root element built *into* the spawned root entity (`spawn_bsx_entry`), so the route tree, server, and middleware all live where they expect to.
- The `{(..)}` spread stacks middleware components onto the router entity, exactly as a Rust `world.spawn((Router, RequestLogger, ..))` would: request logging, `--help`, terminal link navigation, and the layout.
- `BsxLayout{template:"Layout"}` is render middleware like `BaseLayout`, but the document is a BSX template resolved by name from the registry, so markup can choose it. Every page's body transcludes into the template's default `<Slot/>` by reference.
- `<RoutesDir src="routes"/>` scans the directory at spawn time and creates one `BlobScene` route per content file (`.md`/`.bsx`/`.html`), served through the shared media-parse pipeline. `index.*` collapses to its directory. Frontmatter is read at scan time into `ArticleMeta` so navigation knows every page's title and order without visiting it.
- `templates/Layout.bsx` composes registered Rust widgets (`<RouteHead>`, `<Header>`, `<RouteSidebar/>`, `<Stylesheet/>`) with plain markup. Site templates resolve by module path, eg `templates/widgets/Card.bsx` is `<widgets::Card>`, with caller content routed into its slots.

The same site renders on the web (full HTML document) and in the terminal (charcell with the same rule set), and exports statically, all through the standard router pipeline.

## Feature parity checklist

Features of the codegen-based `beet_site`, and how the no-code site covers each:

| Feature | beet_site (codegen) | bsx_site (no-code) |
| --- | --- | --- |
| File-based routes | `RouteCollection` scan at codegen time | `RoutesDir` scan at spawn time |
| Markdown pages + frontmatter | `BlobScene` routes emitted by codegen | same `BlobScene`, spawned by `RoutesDir` |
| Rust page handlers (`pages/*.rs`) | codegen emits typed route fns | `.bsx` pages instead; arbitrary Rust handlers stay a codegen feature |
| Typed route tree (`routes::docs::index()`) | codegen module | not applicable: links are plain hrefs |
| Base layout wrapping every route | `BaseLayout::<BeetLayout>` (generic over a Rust type) | `BsxLayout { template }` (BSX template by registry name) |
| Per-page title/description in `<head>` | layout queries `ArticleMeta`, passes `<Head>` props | `<RouteHead>`: the same lookup as a registered widget |
| Sidebar from route tree | `BeetSidebar` Rust template with hardcoded `with_info` calls | `<RouteSidebar/>`: labels/order/expansion from each page's frontmatter |
| Sidebar active link + auto-expansion | `SidebarState` against current path | same, via `RequestContext` |
| Router middleware (logging, `--help`, navigate) | `default_router()` bundle | component spreads in `main.bsx` |
| Styling: design tokens, Material rules | `MaterialStylePlugin` + `<Stylesheet/>` | same plugin (host) + `<Stylesheet/>` in `Layout.bsx` |
| Preflight/reset/color scheme | layout widgets | same widgets in `Layout.bsx` |
| 404 + contextual help | `Router` fallback + `HelpHandler` | same |
| `/app-info` + `POST /analytics` routes | `default_router()` | not wired (host could spawn them; both need `PackageConfig`) |
| Static site export | launch step | `export` route wired by the host (`site export`) |
| Multi-target rendering (web + terminal) | `--features web/terminal/cli` | same pipeline; CLI mode renders ANSI |
| Syntax highlighting in markdown | `beet/syntax_highlighting` feature | same feature, opt-in |
| Server actions + client islands | codegen (`actions.rs`, `client_actions.rs`) | out of scope: these need Rust handlers, the codegen workflow's domain |
| Package metadata | `pkg_config!()` resource | same, inserted by the host |
| Draft pages | `draft = true` frontmatter | parsed into `ArticleMeta`; filtering still TODO both sides |

## New machinery this spike introduced

In `beet_router`:

- `RoutesDir` + `SiteRoot` (`scene_routes/routes_dir.rs`): runtime route discovery. An observer scans the directory, inserts a `BlobStore` rooted there, and spawns one `BlobScene` route child per content file, with scan-time `ArticleMeta` from frontmatter.
- `BsxLayout` (`router/bsx_layout.rs`): the markup-declared counterpart of `BaseLayout`, resolving the layout from the `BsxTemplateRegistry` by name. Shares the transclusion path via the extracted `wrap_content_with`.
- `RouteHead` + `RouteSidebar` (`router/sidebar.rs`): route-aware widgets registered by name for BSX use.
- End-to-end test: `tests/bsx_site.rs` builds this example's shape from fixtures and asserts the rendered document.

In `beet_core`:

- `spawn_bsx_entry` (`bsx/entry.rs`): spawn a `.bsx` file as an app entrypoint, its single root element built into the returned entity.
- Spreads now apply on every tag kind (`bsx/resolve.rs`): previously `<Router {(..)}/>` silently dropped its spread because only lowercase elements applied them.
- `Option` auto-wrap in BSX reflection (`bsx/reflect.rs`): `title="x"` coerces into an `Option<String>` field as `Some("x")`.
- Recursive type schemas (`value_schema/from_type_info.rs`): a self-referential type (eg `SidebarNode`) lowers to a `ValueSchema::Reference` instead of overflowing the stack, which registering `Sidebar` as a template surfaced.

In `beet_ui`:

- `register_widget_templates` (`widgets/mod.rs`): registers the widget set by name, called by `BsxDefaultsPlugin`, so BSX tags resolve to `Head`, `Header`, `Sidebar`, `Stylesheet`, ...
- `Frontmatter::extract` (`parse/markdown/frontmatter.rs`): standalone leading-fence extraction for scan-time metadata readers.
- The HTML renderer now emits `<!DOCTYPE html>` for doctype nodes (it previously emitted `<!html>`).

## Current limitations and future work

- `beet run <site-dir>`: promote `main.rs` into the CLI so a site needs zero Rust files.
- BSX template props are validate-only: a `.bsx` template cannot yet interpolate its props (`{title}`), so site templates compose via slots (see `widgets/Card.bsx`). Prop binding into the template body is the natural next step.
- Site config in markup (title/description/theme color) rather than `pkg_config!()`.
- A BSX-declarable `HttpServer` (the component is not yet `Reflect`).
- Live reload: re-scan `RoutesDir` and re-register templates on file change (`FsWatcher` exists).
- Draft filtering on export, and `/app-info`/`analytics` parity if wanted.
