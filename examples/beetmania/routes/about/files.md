+++
title = "The Files"

[RouteMeta]
order = 0
+++

# The files

A beet site is a directory:

```
beetmania/
  main.bsx      the entrypoint: the router and its middleware
  templates/    the site's own BSX templates, eg Layout.bsx
  routes/       the content: every file is a page
```

The entrypoint declares the served site as a router:

```html
<Router {(RequestLogger, HelpHandler, Layout{template:"Layout"})}>
	<RoutesDir src="routes"/>
</Router>
```

The capitalized tags resolve to beet's registered components and templates by name, and the `{..}` spread stacks middleware components onto the router entity, exactly as a Rust `world.spawn((Router, RequestLogger, ..))` would.
