+++
title = "Getting Started"
order = 0
+++

# Getting started

A BSX site is a directory:

```
my-site/
  main.bsx      the entrypoint: the router and its middleware
  templates/    the site's own BSX templates, eg Layout.bsx
  routes/       the content: every file is a page
```

The entrypoint declares the whole app as a single root element:

```html
<Router {(RequestLogger, HelpHandler, BsxLayout{template:"Layout"})}>
	<RoutesDir src="routes"/>
</Router>
```

The capitalized tags resolve to beet's registered components and templates by name, and the `{..}` spread stacks middleware components onto the router entity, exactly as a Rust `world.spawn((Router, RequestLogger, ..))` would.
