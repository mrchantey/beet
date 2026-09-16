# Beet Rendering System

A page is authored once as a target-agnostic scene and rendered to the web (HTML + CSS) and the terminal (charcell ANSI). The target is chosen at the edge, not in the page. Read this before editing pages, widgets, or style rules.

Style is expressed as **semantic classes + design tokens**, never raw CSS, because CSS is web-only. Anything web-only (`<head>`, `<style>`, `<script>`, `@media`, raw CSS) is silently skipped by the terminal.

## The DOM target

The browser is the third sink, driven by the same world as the terminal and the string sink (`render/dom/`, wasm32 only). `DomRenderer` is a `NodeVisitor` twin of `HtmlRenderer`: it creates the node each entity paints as and binds the entity to it with `DomNode` (an element or text node of its own, the element an attribute entity sets, or a document element such as the `<body>` the render target stands in for; `<html>` is transparent, `<head>`/`<script>`/`<style>` are skipped). The bound node carries its entity as the `beetEntity` property, so an event's target resolves to its entity by walking up the DOM. `DomRenderPlugin` runs `sync_dom` in `PostUpdate` after the document chain: a `Changed<Value>` patches a text node's data or a control's `value`/`checked` property (skipped when the DOM already agrees, so a focused control keeps its caret), an attribute entity sets its attribute (only stated attributes are ever touched, so a user-toggled `open` survives), and a `Changed<Children>`/`Changed<Portal>` reconciles the nearest painted element's child list by entity identity: surviving nodes stay the same instances, moves are `insertBefore`, new subtrees paint, and a despawn's `DomNode` hook removes its node. The `DomHost` (beet_router) mounts once nothing under the page carries beet_net's `Loading` or a pending `TemplatePending`, and the mount adopts: the walk and the body's served children advance in lockstep (`Frame` cursors in `dom_renderer.rs`), an element of the same tag and namespace, a text node or a `bx` comment is bound as it stands (its listeners, focus and typed value intact; a text node is split where adjacent text entities parsed as one), a mismatch is replaced in place by a fresh paint, surplus served nodes go, and a skipped `<script>`/`<style>` the page carries is bound so a reconcile keeps it. Only stated attributes are compared and set; an adopted control's value is read, never written: what differs from its served default (`value` vs `defaultValue`, `checked` vs `defaultChecked`, an option's `selected` vs `defaultSelected`) is written into the world as `LiveValue::edited`, so a value typed before the wasm arrived needs no replay. The mount returns an `Adoption` (`adopted`, `replaced`, `patched`), logged as `dom host painted <url> (N adopted, 0 replaced, 0 patched)`: the conformance measure the browser suites assert on every served page. A control's value is markup on both sinks (`value=`, `selected`, textarea content), so SSR and the paint agree; the parity probe (`just test-wasm-browser beet_ui --features template_serde`) pins the DOM's `innerHTML` against the string sink's output parsed by the browser, and the conformance probe adopts that parsed output with nothing replaced.

The input direction is `DomInputPlugin` (`render/dom/dom_input.rs`), composed by `LivePagePlugin` on wasm beside the sink. A mount root (an entity `DomRenderer::mount` bound to a document element it did not paint, ie the `DomHost`) installs one set of delegated listeners on that element (`DomListeners`, one `HtmlEventListener` queue so every kind drains in firing order), and `First` drains them into the events the terminal bridge emits, the mount root standing in as the pointer: `click` is `PointerDown` then `PointerUp` on the target's entity (the composed event, so Enter on a `<button>` and a programmatic `click()` count), `pointerover`/`pointerout` are `PointerOver`/`PointerOut`, `input`/`change` write a control's live value into its `Value` through `Value::edit_text` (kind-preserving, no change flagged when equal), `focusin`/`focusout` insert and remove `Focus` (the world mirrors the document's focus, never drives it), Enter/Space on a focusable with no native activation (a tree row) is the pointer pair with its default prevented, and `submit` is prevented at dispatch and lands `Submit` only when no button submitted it (the button's click already did). The focus model (`Focus`, `Focusable`, `FocusPlugin`) is ungated; the key routing (`Tab`, Enter activation, text entry) is `input/keyboard.rs` behind `keyboard`, which the web never enables. A `DomHost` surface leaves anchors to the browser: `on_link_click` returns early on it, a link being a full page load this leg. What the page did before the world existed is taken at the mount: the document's focused element takes `Focus`, and the `click`s and `submit`s the page's pre-boot script queued (`PreBoot`, `render/pre_boot.rs`, ungated: `<Wasm>` renders `PreBoot::script` as a classic inline head script, one capture listener on the window queueing them under `globalThis.beetPreBoot`, `submit` prevented) are dispatched through the same table, a click on a checkbox or radio excepted since adoption read its `checked`. The same script hides the body for a returning editor (`localStorage[StoreUri::fork_mark(repo)]`, which `StoreFork` sets on its first write; `DomHost` calls `PreBoot::reveal` after the paint) and, on `<Wasm boot="intent">`, injects the module loader on the first click or toggle of an element marked `data-beet-boot` (`PreBoot::INTENT`, which `ToggleSceneEditor`'s `<details>` carries), at once for a returning editor. The browser tests live beside the code (`just test-wasm-browser beet_ui --features template_serde`); `tests/bsx_site_browser.rs` (`just test-bsx-site-browser`) proves every route adopts untouched and the counter counts in a served page, `tests/scene_editor_browser.rs` (`just test-scene-editor-browser`, the terminal suite's twin over the shared `tests/browser_host` and its trusted gestures: `WebElement::type_text` appends at the caret's end, `type_ahead` picks a native select's option by prefix, `press` a key on a script-focused element) that the editor's loop (retype mid-keystroke with the caret pinned through `selectionStart`, add, reparent, remove) survives a reload from the fork in IndexedDB, that a pre-boot click lands after the boot, that the boot is invisible (screenshots and `innerHTML` before and after compare equal) and that a returning editor never sees the published text.

## Authoring

A page module exposes `pub fn get() -> impl Bundle` and builds its body with `rsx!`. Markdown files are routes directly, with `+++ ... +++` TOML frontmatter (`title`, `description`, `created`).

```rust
pub fn get() -> impl Bundle {
    rsx! { <article><h1>"Title"</h1><p>"Body"</p></article> }
}
```

A ```` ```mermaid ```` fence (or `<Mermaid src=".."/>`, `<Mermaid>graph LR; A --> B</Mermaid>`) is a diagram on every sink, token-painted: inline svg on the web, box art or a kitty raster on the terminal, the mode the `diagram-render` cascade property (a fence info word, `bx:style`, a `<Rule>`). Cheatsheet: `crates/beet_ui/src/parse/mermaid/mod.rs`; showcase `site/routes/docs/design/diagrams.md`.

## Classes and tokens

Classes are the contract between widgets (emit them) and the rule set (styles them). Never `class="..."` strings; emit semantic classes via the `Classes` block attribute. Constants live in `beet_ui::prelude::classes` (full list: `crates/beet_ui/src/token/classes.rs`); rules mapping them to tokens live in `crates/beet_ui/src/style/material/classes/*.rs`; design tokens (`material::colors::Primary`, `Surface*`, `Outline`, …) in `style/material/colors.rs`, reached downstream through the `material::` prefix (Material is one styling system among potentially many, so `colors` is not re-exported bare).

```rust
<div {Classes::new([classes::CARD_FILLED])}>...</div>
```

Families: `BTN*`, `CARD_*`, the type scale (`TEXT_DISPLAY/HEADLINE/TITLE/BODY/LABEL_*`), `SHAPE_*`, `ELEVATION_*`, layout (`PAGE`, `CONTAINER`, `APP_BAR`, `TABLE`, `SIDEBAR*`), form (`INPUT*`, `SELECT*`, `CHECKBOX`, `ERROR_TEXT`), scheme (`LIGHT_SCHEME`/`DARK_SCHEME`, on an ancestor), utilities (`HIDDEN`, `PRINT_HIDDEN`, `TEXT_LEFT/CENTER/RIGHT`, `TEXT_XS..2XL`).

## Widgets

`#[template]` function components used as capitalized tags, imported via `crate::prelude::*` / `beet::prelude::*`. Source: `crates/beet_ui/src/widgets/`.

- `Button`/`IconButton`/`Link` take their content as the **default slot** (`<Button>"Save"</Button>`), plus `variant: ButtonVariant` (`Filled` default, `Outlined`, `Text`, `Tonal`, `Elevated`, `Secondary`, `Tertiary`, `Error`); `Button` also `action` (renders `type="button"`, so it does not submit the form it sits in); `Link` also `href`. `ErrorText` takes `message`.
- `TextField`/`TextArea`/`NumberField`/`Select`/`Form` take `name`/`field`/`placeholder` options and a `variant`; `TextField` also `sensitive` (masked), `NumberField` also `min`/`max`/`step`; `Select`/`Form` take slot children. `Checkbox` is the boolean control (`name`/`field`), the only one that writes a `Value::Bool`; it paints a `[x]`/`[ ]` marker on the terminal and mirrors `checked` in HTML. All of them are registered by `FormPlugin`.
- `DynamicForm`/`DynamicView` are the write and read halves of the same schema walk, both taking a `schema` plus the `field` they bind under. The form emits a `<form>` with one control per editable leaf: a struct or tuple recurses into a `<details>` group, a list or map into one control per item with add/remove buttons, and a payload-carrying enum into a variant `<select>` plus the selected payload's own controls. Only a kind no control can produce a value for (`Entity`, `Any`, an unresolved reference) renders read-only, marked `UneditableField`. The view emits `key: value` lines, a struct as a `<strong>` title between `<hr>` rules, and a list of structs as a table whose columns are the item's fields.
- A schema-driven widget has three grains of reactivity, and they are independent: a leaf's value through its own binding, the layout the *schema* decides through `SchemaRebuild` (keyed on the registry), and the controls the *value* decides through `ValueRebuild` (keyed on a list's length, a map's keys, an enum's variant, or the schema a sibling field names). `SchemaEditor` is a `DynamicForm` over `ValueSchema::meta()` bound to a `DraftOf` the schema document, committing through `TypedDocument::commit_schema`.
- `Table` (slots `head`/default/`foot`), `Header` (`nav` slot), `Footer`, `Head`, `PageLayout`, `ContentLayout`, `HtmlDocument`, `PageBreak`.
- Web-only head pieces: `Preflight`, `Reset`, `Stylesheet`, `ColorSchemeScript`.

Slotted children use `slot="name"`.

## Rules, inline classes, escape hatches

The target-agnostic way to add a reusable style is a `Rule` in the rule set (sets tokens, resolves on both targets). See `rsx_site/src/style.rs` `design_row_rule` and `style/material/classes/*.rs`. For a **one-off per-element** style use `inline_class!` (`token/class.rs`): it returns the `InlineClass` component, so it works as a block attribute in both `rsx_direct!` and scene `rsx!`. Scene-rsx gotchas: only the **first** `{..}` after a tag is an attribute (a second is a child), so combine a semantic class and an inline class in one tuple `{(Classes::new([..]), inline_class![..])}`; and the class name is sanitized to `inline-<file>-<line>-<col>` (raw `file:line:col` fails on the web).

Common prop tokens (`common_props`): `BackgroundColor`/`ForegroundColor`, `DisplayProp`, `GapProp`/`ColumnGapProp`/`RowGapProp` (`Length`), `Padding`/`MarginProp` (`Spacing`), `Width`/`Height`/`Min*`/`Max*`, `FlexDirectionProp`, `AlignItemsProp`, `JustifyContentProp`, `ShapeProp`, `ElevationProp`, `CursorProp`, `TransformProp`. Values: `Length::{Px,Rem,Percent,Viewport*}`, `Display`, `Direction`, `Cursor`, `Transform::{None,Rotate(deg)}`, `ListStyle::{Auto,None}`.

A raw `<style>` string is the last resort, web-only. Standing web-only overrides live in `beet_ui/src/widgets/browser/browser_overrides.css` (raw CSS, so it *can* use selectors the rule system can't, eg `nav ul`). `Preflight` is a verbatim Tailwind copy, do not edit.

## Cascade rules to know

- **Media gating.** A `Rule` may carry a `MediaQuery`. The charcell/native cascade applies rules with **no** gate *or* the `Terminal` gate; CSS emits `Screen`/`Print`/`ReducedMotion` and drops `Terminal`. So terminal-only styling gates with `Terminal` ("plain default everywhere + a `Terminal` rule opts the terminal in"); never use `Screen` for "not terminal" (it also drops in print). Genuinely screen-only layout (viewport-fill, sticky) uses `Screen`.
- **Adjacent same-selector rules merge only when their media also matches** (`RuleSet::insert_rule`). A `Screen`-gated `.x` rule right after an ungated `.x` rule stays separate, so its declarations don't leak to every target. Overrides between a base and a target-gated rule work through the cascade's later-wins, not through merging.
- **Inheritance mirrors CSS** (`common_props.rs`): text props inherit (color, font-*, line-height, letter-spacing, text-align, white-space, list-style, visibility); box/layout props do **not** (padding, margin, width/height, border-*, border-radius, box-shadow, gap, outline, display, flex-*, transform). Inheriting a box prop silently compounds down a subtree, that bug is why a nested tree over-indents. The one deliberate exception is the text-decoration trio, modelled as inherited so an underline reaches nested spans.
- **Selectors** model tag/class/attribute/state/`AnyOf`/`AllOf`/`Not` and one **descendant combinator** (`Selector::descendant(ancestor, descendant)`, css `a b`). The combinator is **web-only**: it serializes to CSS but the charcell cascade has no ancestor context, so it never matches there (gate it `Screen`). Use it for `[open]`-reactive child styling, eg the sidebar caret rotates via `details:not([open]) .sidebar-caret { transform: rotate(-90deg) }`. For `+`/`>` combinators, use `browser_overrides.css`.
- **Transcluded content inherits the shell cascade.** Route content is rendered then transcluded into the document layout by reference (`Portal`, no `ChildOf` edge). `RuleSetQuery::parent` treats a `Portal` target as a child of its holder and `resolve_styles` follows holders, so inheritance (eg the dark scheme) crosses the boundary. Regression tests in `material_plugin.rs`.

## Routing and codegen

File-based (`rsx_site/src/launch.rs`): `src/pages/*.rs` → `/*`. A collection can set a base route and allow nested dirs (eg a `src/docs/**` collection → `/docs/**`). Each `.rs` route needs `pub fn get() -> impl Bundle`; `.md` files are routes directly. **After any route add/remove/move, regenerate codegen** or the build fails on a missing module:

```bash
cargo run -p rsx_site --no-default-features --features codegen
```

This rewrites `src/codegen/{pages.rs,docs/mod.rs,blog/mod.rs,route_tree.rs}` (gitignored). Typed paths (`routes::docs::index()`) come from there, and the sidebar nav is auto-collected from the route tree.

## Legacy API (in `.agents/references/beet_old`)

| Legacy | Current |
|---|---|
| `-> impl IntoHtml` | `-> impl Bundle` |
| `#[template]` + `signal` | plain scene; reactivity via the `document` module |
| `class="card-filled"` | `{Classes::new([classes::CARD_FILLED])}` |
| `<Button label="Save"/>` | `<Button>"Save"</Button>` (default slot) |
| `<ErrorText value=.../>` | `<ErrorText message="..."/>` |
| `var(--bt-color-primary)` | tokens (`material::colors::Primary`) via rules |

Live-interactivity demos (counters, live binding) rely on the old signal system; when porting to a static page render the visual variants and drop or TODO the demo.

## Gotchas

- `<li>` is `Display::ListItem`, not `Block`: the web needs `list-item` to keep its bullet/number (Preflight strips markers, `browser_overrides.css` restores `list-style` on `ul`/`ol`), while charcell lays it out as a block and draws the marker via `decorate.rs`. A plain `display: block` on a `<li>` silently drops web markers.
- A border changes an element's footprint on both targets (a terminal border eats a whole cell). To keep variants the same size, *reserve* the border on every variant and only recolour it — eg contained buttons (`button_contained`) carry a fill-matched border so they match the outlined variant.
- Color scheme is decided by the layout (`rsx_site/src/layout.rs`), not the renderer: `?color-scheme=light|dark` (CLI `--color-scheme=`) pins a body class; else the web follows the OS via `color_scheme.js` and a non-html target defaults to `.dark-scheme` (dark prose on a dark terminal would be invisible).
- Syntax highlighting needs `beet/syntax_highlighting`; verify with the no-code `site/`'s `/docs/design/code` page.
- `children!`/`related!` are set operations, clobbering existing relations.
- Use `cross_log!()` / `.xprint()`, never `println!` (silent in wasm).
