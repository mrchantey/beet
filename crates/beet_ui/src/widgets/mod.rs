//! Reusable `#[template]` function components.
//!
//! These emit *semantic* classes via [`Classes`](crate::token::Classes)
//! (never `class="…"` strings); the active rule set (Material Design 3 via
//! `MaterialStylePlugin` today) maps those classes to design tokens.
//!
//! Gated behind the `scene` feature; rendering targets and styling come from
//! the same DOM + rule machinery as parsed HTML.
//!
//! Grouped by what a widget *is*: [`controls`] the authored form controls,
//! [`schema_ui`] the ones a schema generates, [`chrome`] the page frame,
//! [`browser`] the web-target `<style>`/`<script>` emitters, and [`debug`] the
//! ones that surface the running app to whoever is working on it. Everything
//! is re-exported flat, so an author names a widget, not its domain.
//!
//! **Reactive substrate.** State lives in documents:
//! [`TypedFieldRef`](beet_core::prelude::TypedFieldRef) for a single typed atom and
//! [`ReactiveChildren`](beet_core::prelude::ReactiveChildren) for a list field that
//! materializes one child per item. The full loop — native event then document
//! mutation then change-detected rebuild — is proven by `native_event_drives_list`
//! in `document/reactive_children.rs`, with zero render-target coupling.
//!
//! A DOM widget or a `BlobStore`-backed list is a render-target / storage
//! binding layered on top, not a gap in the substrate: a render backend triggers
//! the native events (see `input/pointer.rs`), and an async store can sync
//! `BlobStore::list()` into a `Vec<_>` field via
//! [`AsyncWorld`](beet_core::prelude::AsyncWorld) when that integration is wanted.
//!
//! **Schema-driven widgets.** Three widgets take a
//! [`ValueSchema`](beet_core::prelude::ValueSchema) rather than authored
//! children: [`DynamicForm`] generates one control per editable leaf,
//! [`DynamicView`] the read side of the same walk, and [`SchemaEditor`] edits
//! the *schema* itself — a `DynamicForm` over the meta-schema, bound to a
//! [`DraftOf`](beet_core::prelude::DraftOf) the schema document and committing
//! through
//! [`TypedDocument::commit_schema`](beet_core::prelude::TypedDocument).
//! [`ToggleSchemaEditor`] is the last of those behind a closed disclosure, which
//! is how an app opts into editing its own shape.
//!
//! A form or view that names no schema takes the one its document declares, so
//! a document read out of a store describes its own widgets.
//!
//! Their reactivity has three independent grains: a leaf's value through its own
//! binding, the layout the *schema* decides through [`SchemaRebuild`] (so a
//! committed schema edit regenerates every form and view of that schema), and
//! the controls the *value* decides through [`ValueRebuild`] (a list's rows, a
//! map's entries, an enum's payload, a field whose schema a sibling names).

mod browser;
mod chrome;
// `code_snippet` calls `SyntaxHighlighting`, which is native-only (tree-sitter),
// so mirror its `not(wasm32)` gate.
#[cfg(all(
	feature = "net",
	feature = "syntax_highlighting",
	not(target_arch = "wasm32")
))]
mod code_snippet;
mod controls;
mod debug;
mod schema_ui;
/// Shared harness for the widget tests.
#[cfg(test)]
pub(crate) mod test_ext;
mod toast;
mod widget_plugin;

pub use browser::*;
pub use chrome::*;
#[cfg(all(
	feature = "net",
	feature = "syntax_highlighting",
	not(target_arch = "wasm32")
))]
pub use code_snippet::*;
pub use controls::*;
pub use debug::*;
pub use schema_ui::*;
pub use toast::*;
pub(crate) use widget_plugin::widget_plugin;
// `button::Button` collides with the bevy_ui `Button` that leaks in via the
// `beet_core::prelude` glob in the widget files; the explicit re-export pins
// the public `Button`, and downstream `prelude::Button`, to this crate's widget.
pub use controls::button::Button;
