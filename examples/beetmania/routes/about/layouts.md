+++
title = "Layouts"

[RouteMeta]
order = 2
+++

# Layouts

`Layout(Document("templates/Layout.bsx"))` is render middleware: every page's body is transcluded into the default `<slot/>` of that document, instantiated around the page per request. `Layout(Template("SiteLayout"))` names a rust `#[template]` instead.

Declare one on any route, not just the router, and they nest: an ancestor's layout is the outer one, so `<Route path="blog" {Layout(Document("templates/ArticleLayout.bsx"))}>` gives the blog its own chrome inside the site shell. An inner layout is chrome only, never a second `<html>` document.

The layout composes beet's widget set by name:

- `<RouteHead>` fills the page title and description from frontmatter.
- `<Stylesheet/>` bakes the active design tokens into a `<style>` tag.
- `<RouteSidebar/>` collects the route tree into the collapsible nav rail.
- `<Header>`, `<Footer>` and `<MenuButton/>` are ordinary beet widgets.

The same layout renders on the web and in the terminal: document chrome like `<head>` simply does not paint on charcell targets.
