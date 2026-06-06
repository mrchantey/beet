# Beet Rendering System

How the `beet_ui` rendering system works. Pages are authored once as target-agnostic scenes and rendered to multiple targets (the web as HTML + CSS, the terminal as charcell ANSI). Read this before writing or editing pages, widgets, or style rules.

For the iteration workflow (build/verify loop, sub-agent orchestration, visual verification) see the adjacent `SKILL.md`. For the terminal renderer internals see the adjacent `charcell.md`. For browser automation see the adjacent `playwright.md`.

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

Common prop tokens (`beet_ui::prelude::common_props` / `style`): `BackgroundColor`, `ForegroundColor` (Color), `DisplayProp` (Display), `GapProp`/`ColumnGapProp`/`RowGapProp` (all `Length`), `Padding`/`MarginProp` (Spacing/Length), `Width`/`Height`/`MinWidth`/`MaxWidth`/`MinHeight`/`MaxHeight` (Length), `FlexDirectionProp` (Direction), `AlignItemsProp`, `JustifyContentProp`, `CursorProp` (Cursor, web-only). Values: `Length::{Px,Rem,Percent,ViewportWidth,ViewportHeight,ViewportMin,ViewportMax}`, `Display::{Block,Inline,Flex,None}`, `Direction::{Horizontal,Vertical}`, `JustifyContent`, `AlignItems`, `Cursor::{Auto,Pointer}`, `ListStyle::{Auto,None}` (`ListStyleProp`, inherited, maps to `list-style-type`; the charcell list decorator reads it to suppress markers — see the `nav` rule). Site-local: the private `THEME_COLOR` seed in `beet_site/src/server.rs` (all accents derive from it through the style system, never referenced directly) and the `classes` module (`beet_site/src/classes.rs`, re-exports the library `classes` plus `DESIGN_ROW`; reference site-local additions as `crate::classes::DESIGN_ROW` — a bare `classes::` resolves to the library module via the `beet::prelude` glob).

A raw `<style>` element with a CSS string is the last-resort escape hatch. It is web-only and ignored by the terminal. The site's standing web-only overrides live in `beet_ui/src/widgets/reset.css` (emitted by the `Reset` widget, loaded after `Preflight` — itself a verbatim, do-not-edit Tailwind copy). `reset.css` is full raw CSS, so it *can* use descendant/pseudo selectors the rule system cannot (eg `nav ul { list-style: none }`); it currently restores prose list markers Tailwind strips. Reach for raw CSS only when a token/class/rule cannot express the intent.

Registering a rule (above) is for a class reused across elements. For a **one-off, per-element** style, declare an **inline class** on the element itself (`inline_class!`, `crates/beet_ui/src/token/class.rs`). `inline_class!` returns the `InlineClass` *component* (a component hook registers its rule + adds its sanitized class on insert), so it works as a block attribute in **both** `rsx_direct!` and scene `rsx!`. Two scene-rsx gotchas learned the hard way:
- Only the **first** `{..}` after a tag is a block attribute; a second `{..}` is parsed as a child. To put both a semantic class and an inline class on one element, combine them in a single tuple attribute `{(Classes::new([..]), inline_class![..])}` — tuples lift via `IntoScene` only when *every* element does (components do), or just wrap with a `<div>`.
- The inline class name is sanitized to a valid CSS identifier (`inline-<file>-<line>-<col>`); the raw `file:line:col` form silently fails on the web (browsers read `:` `/` `.` as selector tokens).

Three pitfalls when styling layout:

- Charcell reads the layout enum directly; CSS reads `AsCssValue`. The two can disagree (a value valid for the terminal can serialize to invalid CSS), so verify the generated CSS, not just the charcell render.
- Gaps are unified: `ColumnGapProp`/`RowGapProp` are `Length`, serializing to valid CSS *and* driving charcell. They stay `Length` end to end — `FlexBox.row_gap`/`column_gap` are `Length`, not pre-rounded cells, so a pixel native renderer gets pixels and a viewport gap resolves against the real viewport. The charcell flex engine rounds to whole cells at layout via `FlexBox::row_gap_cells(viewport)`/`column_gap_cells(viewport)` (mirroring how `Spacing` insets convert). `GapProp` is the redundant shorthand. The `LayoutStyle`/`FlexBox` gap builders now take `Length` (`.column_gap(Length::Rem(1.))`), not `u32`.
- Charcell doubles horizontal padding (`tui_inset` in `render/charcell/box_model.rs` does `min.x*=2`), so `1rem` left padding = 2 terminal cells, not 1, and `0.5rem` also rounds to 2 (`round(0.5)=1`, then `*2`). A **1-cell** indent is unreachable through padding alone — it needs an explicit per-depth indent in the widget or a charcell change. Budget for this when sizing terminal indents (sidebar tree, app-bar inset).

## Media gating (web vs print vs terminal)

A `Rule` may carry a `MediaQuery` gate. The cascade (charcell/native) applies rules with **no** gate *or* the `Terminal` gate; CSS serialization emits `Screen`/`Print`/`ReducedMotion` and **drops** `Terminal`. So:
- Terminal-only styling (eg the colored prose headings): gate with `MediaQuery::Terminal`. Never use `Screen` for "not in terminal" — `Screen` also drops in print, breaking printed output. The pattern is "plain default everywhere + a `Terminal` rule opts the terminal in" (see `elements.rs` `heading_color`, `rules.rs` `app_bar_terminal`/`sidebar_link_terminal`).
- Genuinely screen-only layout (viewport-fill, sticky) legitimately uses `Screen`.
- `Selector::any_tag([..])` builds a multi-tag `AnyOf`; the selector model has **no descendant/child combinator**. For per-cell-in-a-container styling, an *inherited* token can fake it on charcell (the cascade walks ancestors), but **CSS `border-width` is not inherited**, so that trick frames the container element on the web instead. The `.table-vertical-borders` column dividers are therefore web-only via a raw `td + td`/`th + th` adjacent-sibling rule in `reset.css` (charcell tables stack vertically with no grid, so vertical dividers don't apply there). When a per-descendant style needs the `+`/`>` combinators, reach for `reset.css`.
- **Transcluded content inherits the shell cascade.** Route content is rendered then transcluded into the document shell *by reference* (`RenderRef`, no `ChildOf` edge), so a naive `ChildOf` ancestor walk stops at the content root and a deep non-inherited token (eg a card's `background-color: SurfaceContainerHighest`) falls back to the light `:root` — the "white card on a dark page". Fixed in two places: `RuleSetQuery::parent` treats a `RenderRef` *target* as a child of its holder (so inheritance crosses the boundary), and `resolve_styles` follows `RenderRef` holders when traversing so already-rendered `.md` content re-resolves under the shell. Regression tests: `material_plugin.rs` `nested_card_inherits_dark_scheme` / `render_ref_content_inherits_dark_scheme`.

## Routing and codegen

Routing is file-based (`crates/beet_site/src/launch.rs`):

- `src/pages/*.rs` map to top-level routes (eg `src/pages/foo.rs` to `/foo`)
- `src/docs/**` map to `/docs/**`, with nested dirs allowed (eg `src/docs/design/widgets/button.rs` to `/docs/design/widgets/button`). Codegen is stale after a dir reorg — regenerate it or the build fails on a missing module path.
- `src/blog/**` map to `/blog/**`

Each `.rs` route file needs `pub fn get() -> impl Scene`. `.md` files are routes directly.

After adding or removing route files you must regenerate the codegen modules:

```bash
cargo run -p beet_site --no-default-features --features codegen
```

This rewrites `src/codegen/{pages.rs,docs/mod.rs,blog/mod.rs,route_tree.rs}` (generated artifacts, gitignored). The generated typed paths in `route_tree.rs` are used as `<Link href=routes::docs::index()/>`, and the sidebar nav is auto-collected from the route tree. Render commands and the build/verify loop live in `SKILL.md`.

## Legacy API you may encounter

Older code (and the previous site's source in `.agents/reference/beet_old`) uses a previous, web-only API. When porting it, translate as follows:

| Legacy | Current |
|---|---|
| `-> impl IntoHtml` | `-> impl Scene` |
| `#[template]` + `client:load` + `signal` | plain scene; reactivity via the `document` module |
| `class="card-filled"` (string) | `{Classes::new([classes::CARD_FILLED])}` |
| raw `<style>` scoped blocks | semantic classes / registered rules (raw `<style>` only as a last resort) |
| `<Button label="Increment"/>` (prop) | `<Button>"Increment"</Button>` (default slot) |
| `<ErrorText value=.../>` | `<ErrorText message="..."/>` |
| `var(--bt-color-primary)` | design tokens (`colors::Primary`) via rules |

Live-interactivity demos (counters, live form binding, select `onchange`) rely on the old signal system. When porting to a static page, render the visual variants (every button/input/select variant, etc) and drop or TODO the live demos.

## Gotchas

- Regenerate codegen after any route change, or the build will not see new pages.
- `Button`/`Link`/`IconButton` take content as their **default slot** (`<Button>"Save"</Button>`), not a `label` prop. `ErrorText` still takes `message`.
- Color scheme: the **document shell** decides it (`shell.rs`), not the renderer. An explicit `?color-scheme=light|dark` (CLI `--color-scheme=`) pins a `.light-scheme`/`.dark-scheme` on `<body>`. Absent that, the web follows the OS via `color_scheme.js`; a non-html target (terminal) has no script, so the shell defaults `<body>` to `.dark-scheme` when `!cx.parts().accepts(MediaType::Html)` (dark prose on a dark terminal would be invisible). `RequestParts::accepts` is the helper. The CLI's default Accept (`AnsiTerm,Text,Markdown,Json`, no Html) drives this.
- Syntax highlighting needs the `beet/syntax_highlighting` feature (in `beet_site`'s `render` feature). It emits `class="hl-<capture>"` spans; the theme rules + the syntax `CssTokenMap` are registered by `StylePlugin` (the token map is required or the web `Stylesheet` errors "no CSS resolver registered for … PunctuationSpecial"). Verify with `docs/design/code`.
- The terminal target silently skips anything web-only (`<head>`, `<style>`, `<script>`, `@media`, CSS), which is why class and token styling is preferred.
- `children!`/`related!` are set operations, clobbering existing relations.
- Use `cross_log!()` or `.xprint()`, not `println!` (which is silent in wasm).
</content>
