+++
title = "Welcome"
description = "A beet site with zero code"
+++

# A site with no code

This whole site is markup: a `main.bsx` entrypoint, a `routes/` directory of pages, and a `templates/` directory of BSX templates. There is no codegen and no Rust authoring, just files.

- **Markdown pages** like this one, with frontmatter driving the title and sidebar.
- **BSX pages** like the [counter](/counter), composing site templates and reactive bindings.
- **One layout** wrapping every route, declared in `templates/Layout.bsx`.

Head over to the [docs](/docs) to see how it fits together, or run this site in your terminal:

```sh
beet serve examples/bsx_site
```
