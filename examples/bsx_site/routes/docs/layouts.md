+++
title = "Layouts"
order = 2
+++

# Layouts

`BsxLayout{template:"Layout"}` is render middleware: every page's body is transcluded into the default `<Slot/>` of the named template, registered from the site's `templates/` directory.

The layout composes beet's widget set by name:

- `<RouteHead>` fills the page title and description from frontmatter.
- `<Stylesheet/>` bakes the active design tokens into a `<style>` tag.
- `<RouteSidebar/>` collects the route tree into the collapsible nav rail.
- `<Header>`, `<Footer>` and `<MenuButton/>` are ordinary beet widgets.

The same layout renders on the web and in the terminal: document chrome like `<head>` simply does not paint on charcell targets.
