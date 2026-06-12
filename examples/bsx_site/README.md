# BSX Site

A site declared entirely in markup. `main.bsx` is the entrypoint, `routes/` is the content, `templates/` holds the site's own BSX templates. No Rust authoring and no codegen.

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

Because the site is runtime files, edits to `main.bsx`, the templates, and the routes need no rebuild, just rerun (or re-request, in HTTP mode).

## Layout

```
bsx_site/
  main.rs        a thin host: plugins + package config, then spawn main.bsx
  main.bsx       the entrypoint: the router and its middleware
  templates/     the site's BSX templates, eg Layout.bsx and widgets/Card.bsx
  routes/        the content: every file is a page
```

`main.rs` is a generic host (the shape a future `beet run main.bsx` would take): it adds the server plugins, registers the template directory, then spawns the entry file and layers the host concerns (server backend, dev `export` route) onto it.

## How it works

`main.bsx` declares the whole app as a single root element:

```html
<Router {(RequestLogger, HelpHandler, NavigateHandler, BsxLayout{template:"Layout"})}>
	<RoutesDir src="routes"/>
</Router>
```

- `<Router>` is the `beet_router` dispatch component. The entry's single root element is built *into* the spawned root entity, so the route tree, server, and middleware all live where they expect to.
- The `{(..)}` spread stacks middleware components onto the router entity, exactly as a Rust `world.spawn((Router, RequestLogger, ..))` would: request logging, `--help`, terminal link navigation, and the layout.
- `BsxLayout{template:"Layout"}` is render middleware: every page's body transcludes into the default `<Slot/>` of the `templates/Layout.bsx` template, resolved from the registry by name.
- `<RoutesDir src="routes"/>` scans its directory at spawn time and creates one route per content file (`.md`/`.bsx`/`.html`), served through the shared media-parse pipeline. `index.*` collapses to its directory, and markdown frontmatter is read at scan time so navigation knows every page's title and order.

`templates/Layout.bsx` composes registered widgets (`<RouteHead>`, `<Header>`, `<RouteSidebar/>`, `<Stylesheet/>`) with plain markup. Site-local templates resolve by module path, eg `templates/widgets/Card.bsx` is `<widgets::Card>`, with caller content routed into its slots (see `routes/counter.bsx`).

The same site renders on the web (a full HTML document), in the terminal (charcell with the same style rules), and exports statically, all through the standard router pipeline.
