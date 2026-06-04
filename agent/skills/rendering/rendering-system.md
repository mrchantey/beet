# Beet Rendering System

How the `beet_ui` rendering system works. Pages are authored once as target-agnostic scenes and rendered to multiple targets (the web as HTML + CSS, the terminal as charcell ANSI). Read this before writing or editing pages, widgets, or style rules.

For the iteration workflow (build/verify loop, sub-agent orchestration, visual verification) see the adjacent `iterate-rendering.md`. For the terminal renderer internals see the adjacent `charcell.md`. For browser automation see the adjacent `playwright.md`.

## The big picture

A page is authored once and rendered to any target. The render target (web document shell + HTTP, or the charcell terminal) is selected at the edge, not in the page.

The goal is to keep authoring target-agnostic. CSS is web-only and does not translate to the terminal, so style is expressed through a semantic class plus design-token system rather than raw CSS. Anything web-only (`<head>`, `<style>`, `<script>`, `@media` rules, raw CSS) is silently skipped by the terminal target.

## Authoring a page

A page module exposes `pub fn get() -> impl Scene` and builds its body with `rsx!{ ... }`.

```rust
use crate::prelude::*;
use beet::prelude::*;

pub fn get() -> impl Scene {
    rsx! {
        <article>
            <h1>"Title"</h1>
            <p>"Body text"</p>
        </article>
    }
}
```

Markdown files (`.md`) are also valid routes, with `+++ ... +++` TOML frontmatter (`title`, `description`, `created`).

```md
+++
title = "My Page"
+++
# Heading
body
```

## Classes and tokens

Classes are the contract between the widgets that emit them and the rule set that styles them. Never use `class="..."` strings; emit semantic classes via the `Classes` component block attribute.

```rust
<div {Classes::new([classes::CARD_FILLED])}>...</div>
<h1 {Classes::new([classes::TEXT_DISPLAY_MEDIUM])}>...</h1>
```

Class constants live in `beet_ui::prelude::classes` (re-exported via `beet::prelude::*` and `crate::prelude::*`). The full vocabulary is in `crates/beet_ui/src/token/classes.rs`. Highlights:

- Buttons: `BTN`, `BTN_FILLED`, `BTN_OUTLINED`, `BTN_TEXT`, `BTN_TONAL`, `BTN_ELEVATED`, `BTN_SECONDARY`, `BTN_TERTIARY`, `BTN_ERROR`, `BTN_ICON`
- Cards: `CARD_FILLED`, `CARD_ELEVATED`, `CARD_OUTLINED`
- Typography scale: `TEXT_DISPLAY_LARGE/MEDIUM/SMALL`, `TEXT_HEADLINE_*`, `TEXT_TITLE_*`, `TEXT_BODY_*`, `TEXT_LABEL_*`
- Shape and elevation: `SHAPE_NONE..SHAPE_FULL`, `ELEVATION_0..ELEVATION_5`
- Layout: `PAGE`, `CONTAINER`, `APP_BAR`, `TABLE`, `SIDEBAR*`
- Form: `INPUT`, `INPUT_OUTLINED/FILLED/TEXT`, `SELECT*`, `ERROR_TEXT`
- Color scheme (apply to an ancestor): `LIGHT_SCHEME`, `DARK_SCHEME`
- Utilities: `HIDDEN`, `PRINT_HIDDEN`, `PAGE_BREAK`, `TEXT_LEFT/CENTER/RIGHT`, `TEXT_XS..TEXT_2XL`

Rules mapping classes to tokens live in `crates/beet_ui/src/style/material/rules.rs`. Design tokens (color roles etc) live in `crates/beet_ui/src/style/material/colors.rs`: `colors::Primary`, `OnPrimary`, `PrimaryContainer`, `Secondary`, `Tertiary`, `Error`, `Surface*`, `Outline`, `InverseSurface`, and the rest of the Material 3 set.

## Widgets

Widgets are `#[scene]` function components used as capitalized tags. Import via `crate::prelude::*` / `beet::prelude::*`. Source: `crates/beet_ui/src/widgets/`. Note that text widgets take their text as a prop, not as children.

- `Button { label: into String, variant: ButtonVariant }`, text via `label`
- `IconButton { label, variant }`
- `Link { label, href: into String, variant: ButtonVariant }`, an `<a>` styled as a button
- `ButtonVariant`: `Filled` (default), `Outlined`, `Text`, `Tonal`, `Elevated`, `Secondary`, `Tertiary`, `Error`
- `TextField { variant: TextFieldVariant, name: Option, placeholder: Option, field: Option<FieldRef> }`
- `TextArea`, same props as `TextField`
- `Select { variant: SelectVariant, name: Option, field }`, plus `<option>` children via the default slot
- `Form { name: Option, field: Option }`, plus slot children
- `TextFieldVariant` / `SelectVariant`: `Outlined` (default), `Filled`, `Text`
- `ErrorText { message: into String }`
- `Table`, slots: `head` (`<tr>` rows), default (body rows), `foot`
- `Header { home_route }` with a `<nav>` slot, plus `Footer`, `Head`, `PageLayout`, `ContentLayout`, `HtmlDocument`, `PageBreak`
- Web-only head pieces: `Preflight`, `Stylesheet`, `ColorSchemeScript`

Slotted children are passed with `slot="name"`, eg `<tr slot="head">...</tr>`.

## Layout, rules, and escape hatches

The proper target-agnostic way to add a reusable style class is to register a `Rule` in the rule set, because it resolves on both web and terminal (it sets design tokens, not raw CSS). Add the rule once at app setup and reference it by class on elements. See `beet_site/src/server.rs` `design_row_rule` for a worked flex-row example, and `style/material/rules.rs` for the library patterns, including token-to-token references via `.with_token(BackgroundColor, colors::Primary)`.

Common prop tokens (`beet_ui::prelude::common_props` / `style`): `BackgroundColor`, `ForegroundColor` (Color), `DisplayProp` (Display), `GapProp`/`ColumnGapProp`/`RowGapProp`, `Padding`/`MarginProp` (Spacing/Length), `Width`/`Height` (Length), `FlexDirectionProp` (Direction), `AlignItemsProp`, `JustifyContentProp`. Values: `Length::{Px,Rem,Percent}`, `Display::{Block,Inline,Flex,None}`, `Direction::{Horizontal,Vertical}`, `JustifyContent`, `AlignItems`.

A raw `<style>` element with a CSS string is the last-resort escape hatch. It is web-only and ignored by the terminal, so reach for it only when a token or class genuinely cannot express what is needed (eg a pure color-palette showcase keyed off the emitted CSS variables). Use it sparingly; it is exactly the target-agnostic property we are trying to preserve.

Two pitfalls when styling layout:

- `inline_class!` and the `OnSpawn` it returns do not work as a scene-mode `rsx!` block attribute. Scene block attributes only accept plain `Component` values (`Default + Clone + Component`), eg `Classes`/`FieldRef`. `inline_class!` is for `rsx_direct!`/bundle mode, not `impl Scene` pages. To style a one-off element in a scene, register a `Rule` and attach its class.
- Flex gaps differ by target. `ColumnGapProp`/`RowGapProp` are `u32` and drive the charcell flex layout, but they serialize to unitless CSS (`column-gap: 2`) which browsers ignore. For the web `gap` to render, also set `GapProp` (a `Length`, eg `Length::Rem(1.0)`, which emits a valid `gap: 1rem`). Set both on a layout rule so each target gets its gap. The library's own `.app-bar-nav`/`footer` rules predate this and only set the u32 props, so their web gap is currently 0, a known framework follow-up.

## Routing and codegen

Routing is file-based (`crates/beet_site/src/launch.rs`):

- `src/pages/*.rs` map to top-level routes (eg `src/pages/foo.rs` to `/foo`)
- `src/docs/**` map to `/docs/**`, with nested dirs allowed (eg `src/docs/design/color/schemes.rs` to `/docs/design/color/schemes`)
- `src/blog/**` map to `/blog/**`

Each `.rs` route file needs `pub fn get() -> impl Scene`. `.md` files are routes directly.

After adding or removing route files you must regenerate the codegen modules:

```bash
cargo run -p beet_site --no-default-features --features codegen
```

This rewrites `src/codegen/{pages.rs,docs/mod.rs,blog/mod.rs,route_tree.rs}` (generated artifacts, gitignored). The generated typed paths in `route_tree.rs` are used as `<Link href=routes::docs::index()/>`, and the sidebar nav is auto-collected from the route tree.

## Running both targets

```bash
# web server, serves on 127.0.0.1:8337
cargo run -p beet_site --features web

# terminal (charcell) render of a single route, path segments after --
cargo run -p beet_site --features cli -- blog post-1
cargo run -p beet_site --features cli -- docs design color schemes
```

CLI mode renders one route to stdout (HTML or ANSI per `--accept`) and exits.

```bash
cargo test -p beet_site                       # end-to-end render tests (custom harness)
cargo test -p beet_ui --lib render::charcell  # charcell unit and snapshot tests
```

## Legacy API you may encounter

Older code (and the reference mockups in `agent/reference/mockups`) uses a previous, web-only API. When porting it, translate as follows:

| Legacy | Current |
|---|---|
| `-> impl IntoHtml` | `-> impl Scene` |
| `#[template]` + `client:load` + `signal` | plain scene; reactivity via the `document` module |
| `class="card-filled"` (string) | `{Classes::new([classes::CARD_FILLED])}` |
| raw `<style>` scoped blocks | semantic classes / registered rules (raw `<style>` only as a last resort) |
| `<Button>Increment</Button>` (children) | `<Button label="Increment"/>` (prop) |
| `<ErrorText value=.../>` | `<ErrorText message="..."/>` |
| `var(--bt-color-primary)` | design tokens (`colors::Primary`) via rules |

Live-interactivity demos (counters, live form binding, select `onchange`) rely on the old signal system. When porting to a static page, render the visual variants (every button/input/select variant, etc) and drop or TODO the live demos.

## Gotchas

- Regenerate codegen after any route change, or the build will not see new pages.
- `Button`/`Link`/`ErrorText` take text as a prop, not as children.
- The terminal target silently skips anything web-only (`<head>`, `<style>`, `<script>`, `@media`, CSS), which is why class and token styling is preferred.
- `children!`/`related!` are set operations, clobbering existing relations.
- Use `cross_log!()` or `.xprint()`, not `println!` (which is silent in wasm).
</content>
