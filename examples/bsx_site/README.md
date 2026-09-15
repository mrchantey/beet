# BSX Site

A site declared entirely in markup. `main.bsx` is the entrypoint, `routes/` is the content, `templates/` holds the site's own BSX templates. No Rust authoring, no codegen, and no `main.rs`: the `beet` binary discovers `main.bsx`, and the `CallOnReady` verb it declares boots the servers the moment the entry loads.

```sh
# run from the site dir so the binary discovers its main.bsx (or pass --main=<path>)
cd examples/bsx_site

# CLI render mode: the home route, or a named route
beet --server=cli
beet --server=cli docs/getting-started

# HTTP server
beet --server=http

# live terminal
beet --server=tui

# static export to dist/ (a dev command, run from the repo)
beet export-static examples/bsx_site

# the browser boots the same entry from the served page (see Layout.bsx),
# on the wasm binary this installs (run from the repo)
just build-wasm-ui
```

Because the site is runtime files, edits to `main.bsx`, the templates, and the routes need no rebuild, just rerun (or re-request, in HTTP mode). Install the CLI with `cargo binstall beet-cli`, or from a checkout `cargo install --path crates/beet-cli` (this site needs only default features).

## Layout

```
bsx_site/
  main.bsx       the entrypoint: the router and its middleware
  templates/     the site's BSX templates, eg Layout.bsx and widgets/Card.bsx
  routes/        the content: every file is a page
```

There is no `main.rs`. The `beet` binary discovers `main.bsx`, registers the sibling `templates/` directory, roots the repo store at this directory (which `<RoutesDir/>` resolves against), and loads `main.bsx` as the app root. The `<CallOnReady>` verb declared on the root then boots the servers once the entry has loaded: `--server=http` starts the HTTP listener, `--server=cli` runs one render, `--server=tui` opens the live terminal, `--server=dom` paints the page in a browser tab. `<DefaultAppRoutes/>` layers the default app routes (`/app-info`, `POST /analytics`) on. Static export is a separate dev command, `beet export-static <site-dir>`. The `<PackageConfig/>` declared in `main.bsx` supplies the site title and description those routes read.

## How it works

`main.bsx` declares the whole app as a single root element:

```html
<CallOnReady {(HttpServer, TuiServer, SshTuiServer, DomServer, CliServer)}>
<Router {(RequestLogger, HelpHandler, NavigateHandler, Layout{template:"Layout"})}>
	<PackageConfig title="BSX Site" description="A beet site with zero code"/>
	<RoutesDir src="routes"/>
	<ServeBlobs prefix="repo"/>
	<AssetsDir src="assets" prefix="assets"/>
</Router>
</CallOnReady>
```

- The servers own the root: `HttpServer`, `TuiServer`, `SshTuiServer`, `DomServer` and `CliServer` are transport components, and `CallOnReady` is the boot verb: on load it calls them with the process request, and `--server` selects which boot, so the same markup serves every target with no host binary. A server a binary lacks (`SshTuiServer` in a lean build, `HttpServer` in the browser) is skipped with a warning and the entry keeps its shape.
- `<Router>` is the `beet_router` dispatch component, the servers' child. The route tree and the middleware live on it.
- `<ServeBlobs prefix="repo"/>` publishes the site's own directory read-only at `/repo`, the store the browser process reads this entry through; `<AssetsDir>` mounts the wasm binary the page boots.
- `<PackageConfig/>` is a resource declaration: a capitalized tag naming a `#[reflect(Resource)]` type patches the live resource's named fields (here the site title and description), leaving the rest, eg the compile-time version, untouched. It produces no markup.
- The `{(..)}` spread stacks middleware components onto the router entity, exactly as a Rust `world.spawn((Router, RequestLogger, ..))` would: request logging, `--help`, terminal link navigation, and the layout.
- `Layout{template:"Layout"}` is render middleware: every page's body transcludes into the default `<Slot/>` of the `templates/Layout.bsx` template, resolved by name exactly as the tag `<Layout/>` would be (a `.bsx` file, else a rust `#[template]`). Declared on a child route instead, it nests inside its ancestors', giving a subtree its own chrome.
- `<RoutesDir src="routes"/>` scans its directory at spawn time and creates one route per content file (`.md`/`.bsx`/`.html`), served through the shared media-parse pipeline. `index.*` collapses to its directory, and markdown frontmatter is read at scan time so navigation knows every page's title and order.

`templates/Layout.bsx` composes registered widgets (`<RouteHead>`, `<Header>`, `<RouteSidebar/>`, `<Stylesheet/>`) with plain markup. Site-local templates resolve by module path, eg `templates/widgets/Card.bsx` is `<widgets::Card>`, taking props as tag attributes and routing caller content into its `<Slot/>` (see `routes/counter.bsx`).

## Bindings

All interpolation is reactive and source-prefixed with `@` (the canonical grammar reference is the `beet_core::bsx` module doc). The site uses each source:

- `@doc:` document state: the counter page binds `{@doc:count=0}` and the buttons mutate it from a one-line event script, `bx:click="target.with_field('count', count => count + 1)"`.
- `@res:` resource fields: the footer pulls `@res:PackageConfig.description` straight from the resource (the default document head emits `og:site_name` from `PackageConfig.title` automatically, so the layout no longer hand-writes it).
- `@prop:` template props: `widgets::Card` binds its heading to `{@prop:title}`, filled by the caller's `title="Counter"` attribute.

HTML responses render with the bindings settled, while live targets (the terminal today) keep syncing them continuously.

The same site renders on the web (a full HTML document), in the terminal (charcell with the same style rules), and exports statically, all through the standard router pipeline.

## One counter, three targets

`routes/counter.bsx` is a single no-code page that runs on every target, unchanged:

- **Terminal (in-process):** `beet --server=tui` drives the same `@doc:count` and event scripts natively through the document sync, the count repainting in charcell on each click.
- **Web (the browser runtime):** `beet --server=http` serves the page as its first paint, and the `<Wasm/>` in `templates/Layout.bsx` boots the wasm `beet` binary on this same entry, read through `/repo`, under `--server=dom`. The tab then runs the page's world: a click reaches the button's entity through the DOM binding, its `bx:click` script runs in the sandboxed iframe backend, and the bound count repaints in place with no network round-trip. Until it boots, the served page is inert.
- **Static export:** `beet export-static examples/bsx_site` writes the page with its bindings settled to the initial state, the correct first paint.

There is no framework tier between SSR and the world: below wasm the page is plain HTML and CSS, and above it the browser is one more beet runtime, launched by the page.
