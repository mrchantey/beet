+++
title = "Routing"
order = 1
+++

# Routing

`<RoutesDir src="routes"/>` scans its directory when spawned and creates one route per content file, mirroring the file tree:

| file | route |
| --- | --- |
| `routes/index.md` | `/` |
| `routes/docs/index.md` | `/docs` |
| `routes/docs/routing.md` | `/docs/routing` |
| `routes/counter.bsx` | `/counter` |

Markdown, HTML and BSX files all parse through the same media pipeline, so a page can be prose, markup, or a mix. Frontmatter is read at scan time, which is how the sidebar knows every page's title and order without visiting it:

```toml
+++
title = "Routing"
order = 1
+++
```
