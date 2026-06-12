# BSX Site

A site declared entirely in markup. `main.bsx` is the entrypoint, `routes/` is the content, `templates/` holds the site's own BSX templates. No Rust authoring and no codegen.

```sh
alias site='cargo run --example bsx_site --features "http_server,client_io,json,markdown,style,template,fs" --'

# CLI mode: render the home route, or a named route
site
site docs/getting-started

# HTTP mode, optionally watching the site dir for live reload
site --server=http
site --server=http --watch

# static export to examples/bsx_site/dist
site export
```

Because the site is runtime files, edits to `main.bsx`, the templates, and the routes need no rebuild, just rerun (or re-request, in HTTP mode).

## Live reload

`--watch` spawns a `LiveReload` watcher on the site dir: on any change it re-registers the `templates/` directory, re-scans every `RoutesDir` (despawning and respawning its route children, rebuilding the route tree), and broadcasts `reload` over the `ClientIo` websocket channel. The channel runs beside the HTTP server on its own port since none of the HTTP backends support websocket upgrades.

The browser side is the `<LiveReloadScript/>` widget in the layout head: it connects to the channel, reloads the page on a `reload` message, and reconnects with exponential backoff, reloading after the server itself restarts. The widget renders nothing when no channel is active, so it is safe to leave in the layout for production and static export. The same applies to `beet run <site-dir> --server=http --watch`.

## Layout

```
bsx_site/
  main.rs        a thin host: plugins + package config, then spawn main.bsx
  main.bsx       the entrypoint: the router and its middleware
  templates/     the site's BSX templates, eg Layout.bsx and widgets/Card.bsx
  routes/        the content: every file is a page
```

`main.rs` is a generic host (the documented reference for what `beet run <site-dir>` does): it adds the server plugins, registers the template directory, then spawns the entry file and layers the host concerns (server backend, dev `export` route) onto it.

## How it works

`main.bsx` declares the whole app as a single root element:

```html
<Router {(RequestLogger, HelpHandler, NavigateHandler, BsxLayout{template:"Layout"})}>
	<PackageConfig title="BSX Site" description="A beet site with zero code"/>
	<RoutesDir src="routes"/>
</Router>
```

- `<Router>` is the `beet_router` dispatch component. The entry's single root element is built *into* the spawned root entity, so the route tree, server, and middleware all live where they expect to.
- `<PackageConfig/>` is a resource declaration: a capitalized tag naming a `#[reflect(Resource)]` type patches the live resource's named fields (here the site title and description), leaving the rest, eg the compile-time version, untouched. It produces no markup.
- The `{(..)}` spread stacks middleware components onto the router entity, exactly as a Rust `world.spawn((Router, RequestLogger, ..))` would: request logging, `--help`, terminal link navigation, and the layout.
- `BsxLayout{template:"Layout"}` is render middleware: every page's body transcludes into the default `<Slot/>` of the `templates/Layout.bsx` template, resolved from the registry by name.
- `<RoutesDir src="routes"/>` scans its directory at spawn time and creates one route per content file (`.md`/`.bsx`/`.html`), served through the shared media-parse pipeline. `index.*` collapses to its directory, and markdown frontmatter is read at scan time so navigation knows every page's title and order.

`templates/Layout.bsx` composes registered widgets (`<RouteHead>`, `<Header>`, `<RouteSidebar/>`, `<Stylesheet/>`) with plain markup. Site-local templates resolve by module path, eg `templates/widgets/Card.bsx` is `<widgets::Card>`, taking props as tag attributes and routing caller content into its `<Slot/>` (see `routes/counter.bsx`).

## Bindings

All interpolation is reactive and source-prefixed with `@` (the canonical grammar reference is the `beet_core::bsx` module doc). The site uses each source:

- `@doc:` document state: the counter page binds `{@doc:count=0}` and the buttons mutate it via the event verbs, `bx:click="increment@doc:count"`.
- `@res:` resource fields: the layout pulls `@res:PackageConfig.title` into the head's `og:site_name` meta and `@res:PackageConfig.description` into the footer.
- `@prop:` template props: `widgets::Card` binds its heading to `{@prop:title}`, filled by the caller's `title="Counter"` attribute.

HTML responses render with the bindings settled, while live targets (the terminal today) keep syncing them continuously.

The same site renders on the web (a full HTML document), in the terminal (charcell with the same style rules), and exports statically, all through the standard router pipeline.
