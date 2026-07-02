# No-code sites (pure `.bsx`/`.mdx`)

A site can be authored entirely in markup — no crate, no `main.rs`, no codegen — and served by the prebuilt `beet` CLI (`beet serve <dir>`). `site/` is the real example; `examples/bsx_site/` is the small teaching one. The same parser, widget library, Material rule set, and multi-target renderer back it as a compiled site.

## The host (`main.bsx`)
A `<Router>` whose middleware/server attach as component spreads, then the site config as child tags. The whole site, declared in markup:
```html
<Router {(RequestLogger, HelpHandler, NavigateHandler, BsxLayout{template:"Layout"}, HttpServer{port:8339})}>
  <PackageConfig title="Beet" description="..." version="0.0.9-dev.16"/>
  <Theme color={Srgba(Srgba{red:0.0,green:1.0,blue:0.75,alpha:1.0})}/>
  <Styles/>            <!-- templates/Styles.bsx: the site's <Rule>s -->
  <DefaultAppRoutes/>  <!-- /app-info, /assets/*, the reactivity runtime, ... -->
  <RoutesDir src="routes"/>   <!-- file-based routing, scans frontmatter -->
  <AssetsDir src="assets"/>   <!-- static file mount (favicon, images) -->
</Router>
```
`templates/Layout.bsx` is a thin `<SiteLayout/>` (the shipped shell). No typed `routes::` module exists; internal links are string hrefs, validated by `beet check`.

## Markup analogues of the typed primitives
| Typed (Rust) | No-code (markup) |
|---|---|
| `BeetLayout` doc shell | `<SiteLayout>` (slot-driven; defaults = full beet chrome) |
| `MaterialStylePlugin::new(color)` | `<Theme color={Srgba(Srgba{..})}/>` (patches the live `Theme` resource) |
| a typed `Rule` (eg `design_row_rule`) | `<Rule class="design-row" display=Flex column-gap=Rem(1.0) .../>` in a `styles.bsx` |
| `inline_class![(MaxWidth, Rem(40.))]` | `bx:style="max-width=Rem(40.0)"` on the element |
| `BlobStore`/`serve_store` endpoints | `<RoutesDir>` / `<AssetsDir>` |
| typed route checking (dropped) | `beet check <site>` (broken-href/unknown-tag/unknown-class) |

`<Rule>`/`bx:style` values are the **enum-variant grammar**, identical to the typed API: `Rem(1.0)`, `Flex`, `Vertical` (CSS `column`), `ButtonVariant::Filled`, `Center` — NOT CSS shorthand (`1rem`/`flex`/`column` choke). A token binding is `"@token:Primary"` (quoted). A `Color` literal is the brace-wrapped nested tuple-variant `{Srgba(Srgba{red:..,green:..,blue:..,alpha:..})}`.

## `.mdx` vs `.bsx` (the one that bites)
- `.md`/`.mdx` go through the **fragment** parser (markdown + embedded BSX tags). Markdown prose does NOT interpolate `{...}` — `{@doc:count=0}` renders literally.
- `.bsx` goes through the **document** parser, which DOES interpolate `{...}`.
- So a stateful/reactive page (a `{@doc:...}` binding, the counter) must be a `.bsx`; static content showcases are `.mdx`. Both embed widgets the same way.
- The fragment parser had several gaps the typed/document path didn't (now fixed): unquoted `variant=ButtonVariant::X` / `bool=true` reach typed props via the value grammar; `class={["a","b"]}` list classes; insignificant inter-tag whitespace is dropped (else it paints blank charcell rows); `bx:style`/`bx:<event>` lower like the document parser. An inline class is content-addressed (collision-free across files), not span-addressed.

## Serving a no-code site (the dev loop)
`beet serve` is a loadable scene; after a `beet_ui`/router change the cache goes stale and serve 404s. Refresh it:
```
cargo build -p beet-cli
cargo run -p beet-cli --bin export_scenes && ~/.cargo_target/debug/beet load target/scenes/default-cli.json
~/.cargo_target/debug/beet serve site/ --port=8339          # then git checkout .beet/scene.json
```
A `.bsx`/`.mdx`/`.md` edit needs a server RESTART (RoutesDir scans at startup). `beet check <site>` validates without serving (exit 70 on an error-level diagnostic).

## Parity verification (matching a compiled reference)
When a no-code site must match a compiled one (eg `examples/rsx_site`):
- **Web (pixel gate):** screenshot both, `compare -metric AE golden.png new.png null:` (0 = identical). Drive each diff to AE=0; reduce a diff to its ONE upstream cause (often a cascade across many routes — eg an incomplete sidebar subtree, a missing rule), fix the cause. `.agents/tmp/shot.cjs` shoots one URL.
- **Terminal (charcell gate):** `Accept: text/ansi-term`, strip escapes, diff the stripped `.txt` (layout) — its own gate, different engine; a web fix can break it.
- **Reference integrity:** a framework fix can change the COMPILED site's rendering too (the charcell whitespace/comment corrections did, on the terminal only). The golden snapshot then goes stale; re-capture it from the current reference (which the no-code site, using the same framework, matches). Re-verify the web is unchanged (rebuild the reference, re-diff vs golden — AE must stay 0).
- A `.bsx` page with a leading `<!--comment-->` is multi-root; the layout middleware forwards the body into a template-invocation-root layout (`<SiteLayout/>`) via `anchor_pre_slot_children` (spawn_template.rs).
