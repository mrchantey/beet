# Beet Rendering System

A page is authored once as a target-agnostic scene and rendered to the web (HTML + CSS) and the terminal (charcell ANSI). The target is chosen at the edge, not in the page. Read this before editing pages, widgets, or style rules.

Style is expressed as **semantic classes + design tokens**, never raw CSS, because CSS is web-only. Anything web-only (`<head>`, `<style>`, `<script>`, `@media`, raw CSS) is silently skipped by the terminal.

## Authoring

A page module exposes `pub fn get() -> impl Scene` and builds its body with `rsx!`. Markdown files are routes directly, with `+++ ... +++` TOML frontmatter (`title`, `description`, `created`).

```rust
pub fn get() -> impl Scene {
    rsx! { <article><h1>"Title"</h1><p>"Body"</p></article> }
}
```

## Classes and tokens

Classes are the contract between widgets (emit them) and the rule set (styles them). Never `class="..."` strings; emit semantic classes via the `Classes` block attribute. Constants live in `beet_ui::prelude::classes` (full list: `crates/beet_ui/src/token/classes.rs`); rules mapping them to tokens live in `crates/beet_ui/src/style/material/classes/*.rs`; design tokens (`colors::Primary`, `Surface*`, `Outline`, …) in `style/material/colors.rs`.

```rust
<div {Classes::new([classes::CARD_FILLED])}>...</div>
```

Families: `BTN*`, `CARD_*`, the type scale (`TEXT_DISPLAY/HEADLINE/TITLE/BODY/LABEL_*`), `SHAPE_*`, `ELEVATION_*`, layout (`PAGE`, `CONTAINER`, `APP_BAR`, `TABLE`, `SIDEBAR*`), form (`INPUT*`, `SELECT*`, `ERROR_TEXT`), scheme (`LIGHT_SCHEME`/`DARK_SCHEME`, on an ancestor), utilities (`HIDDEN`, `PRINT_HIDDEN`, `TEXT_LEFT/CENTER/RIGHT`, `TEXT_XS..2XL`).

## Widgets

`#[scene]` function components used as capitalized tags, imported via `crate::prelude::*` / `beet::prelude::*`. Source: `crates/beet_ui/src/widgets/`.

- `Button`/`IconButton`/`Link` take their content as the **default slot** (`<Button>"Save"</Button>`), plus `variant: ButtonVariant` (`Filled` default, `Outlined`, `Text`, `Tonal`, `Elevated`, `Secondary`, `Tertiary`, `Error`); `Link` also `href`. `ErrorText` takes `message`.
- `TextField`/`TextArea`/`Select`/`Form` take `name`/`field`/`placeholder` options and a `variant`; `Select`/`Form` take slot children.
- `Table` (slots `head`/default/`foot`), `Header` (`nav` slot), `Footer`, `Head`, `PageLayout`, `ContentLayout`, `HtmlDocument`, `PageBreak`.
- Web-only head pieces: `Preflight`, `Reset`, `Stylesheet`, `ColorSchemeScript`.

Slotted children use `slot="name"`.

## Rules, inline classes, escape hatches

The target-agnostic way to add a reusable style is a `Rule` in the rule set (sets tokens, resolves on both targets). See `beet_site/src/server.rs` `design_row_rule` and `style/material/classes/*.rs`. For a **one-off per-element** style use `inline_class!` (`token/class.rs`): it returns the `InlineClass` component, so it works as a block attribute in both `rsx_direct!` and scene `rsx!`. Scene-rsx gotchas: only the **first** `{..}` after a tag is an attribute (a second is a child), so combine a semantic class and an inline class in one tuple `{(Classes::new([..]), inline_class![..])}`; and the class name is sanitized to `inline-<file>-<line>-<col>` (raw `file:line:col` fails on the web).

Common prop tokens (`common_props`): `BackgroundColor`/`ForegroundColor`, `DisplayProp`, `GapProp`/`ColumnGapProp`/`RowGapProp` (`Length`), `Padding`/`MarginProp` (`Spacing`), `Width`/`Height`/`Min*`/`Max*`, `FlexDirectionProp`, `AlignItemsProp`, `JustifyContentProp`, `ShapeProp`, `ElevationProp`, `CursorProp`, `TransformProp`. Values: `Length::{Px,Rem,Percent,Viewport*}`, `Display`, `Direction`, `Cursor`, `Transform::{None,Rotate(deg)}`, `ListStyle::{Auto,None}`.

A raw `<style>` string is the last resort, web-only. Standing web-only overrides live in `beet_ui/src/widgets/reset.css` (raw CSS, so it *can* use selectors the rule system can't, eg `nav ul`). `Preflight` is a verbatim Tailwind copy, do not edit.

## Cascade rules to know

- **Media gating.** A `Rule` may carry a `MediaQuery`. The charcell/native cascade applies rules with **no** gate *or* the `Terminal` gate; CSS emits `Screen`/`Print`/`ReducedMotion` and drops `Terminal`. So terminal-only styling gates with `Terminal` ("plain default everywhere + a `Terminal` rule opts the terminal in"); never use `Screen` for "not terminal" (it also drops in print). Genuinely screen-only layout (viewport-fill, sticky) uses `Screen`.
- **Adjacent same-selector rules merge only when their media also matches** (`RuleSet::insert_rule`). A `Screen`-gated `.x` rule right after an ungated `.x` rule stays separate, so its declarations don't leak to every target. Overrides between a base and a target-gated rule work through the cascade's later-wins, not through merging.
- **Inheritance mirrors CSS** (`common_props.rs`): text props inherit (color, font-*, line-height, letter-spacing, text-align, white-space, list-style, visibility); box/layout props do **not** (padding, margin, width/height, border-*, border-radius, box-shadow, gap, outline, display, flex-*, transform). Inheriting a box prop silently compounds down a subtree, that bug is why a nested tree over-indents. The one deliberate exception is the text-decoration trio, modelled as inherited so an underline reaches nested spans.
- **Selectors** model tag/class/attribute/state/`AnyOf`/`AllOf`/`Not` and one **descendant combinator** (`Selector::descendant(ancestor, descendant)`, css `a b`). The combinator is **web-only**: it serializes to CSS but the charcell cascade has no ancestor context, so it never matches there (gate it `Screen`). Use it for `[open]`-reactive child styling, eg the sidebar caret rotates via `details:not([open]) .sidebar-caret { transform: rotate(-90deg) }`. For `+`/`>` combinators, use `reset.css`.
- **Transcluded content inherits the shell cascade.** Route content is rendered then transcluded into the document layout by reference (`RenderRef`, no `ChildOf` edge). `RuleSetQuery::parent` treats a `RenderRef` target as a child of its holder and `resolve_styles` follows holders, so inheritance (eg the dark scheme) crosses the boundary. Regression tests in `material_plugin.rs`.

## Routing and codegen

File-based (`beet_site/src/launch.rs`): `src/pages/*.rs` → `/*`, `src/docs/**` → `/docs/**` (nested dirs allowed), `src/blog/**` → `/blog/**`. Each `.rs` route needs `pub fn get() -> impl Scene`; `.md` files are routes directly. **After any route add/remove/move, regenerate codegen** or the build fails on a missing module:

```bash
cargo run -p beet_site --no-default-features --features codegen
```

This rewrites `src/codegen/{pages.rs,docs/mod.rs,blog/mod.rs,route_tree.rs}` (gitignored). Typed paths (`routes::docs::index()`) come from there, and the sidebar nav is auto-collected from the route tree.

## Legacy API (in `.agents/reference/beet_old`)

| Legacy | Current |
|---|---|
| `-> impl IntoHtml` | `-> impl Scene` |
| `#[template]` + `signal` | plain scene; reactivity via the `document` module |
| `class="card-filled"` | `{Classes::new([classes::CARD_FILLED])}` |
| `<Button label="Save"/>` | `<Button>"Save"</Button>` (default slot) |
| `<ErrorText value=.../>` | `<ErrorText message="..."/>` |
| `var(--bt-color-primary)` | tokens (`colors::Primary`) via rules |

Live-interactivity demos (counters, live binding) rely on the old signal system; when porting to a static page render the visual variants and drop or TODO the demo.

## Gotchas

- Color scheme is decided by the layout (`beet_site/src/layouts/layout.rs`), not the renderer: `?color-scheme=light|dark` (CLI `--color-scheme=`) pins a body class; else the web follows the OS via `color_scheme.js` and a non-html target defaults to `.dark-scheme` (dark prose on a dark terminal would be invisible).
- Syntax highlighting needs `beet/syntax_highlighting` (in `beet_site`'s `render` feature); verify with `docs/design/code`.
- `children!`/`related!` are set operations, clobbering existing relations.
- Use `cross_log!()` / `.xprint()`, never `println!` (silent in wasm).
