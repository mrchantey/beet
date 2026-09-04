//! Reusable `#[template]` function components.
//!
//! These emit *semantic* classes via [`Classes`](crate::token::Classes)
//! (never `class="…"` strings); the active rule set (Material Design 3 via
//! `MaterialStylePlugin` today) maps those classes to design tokens.
//!
//! Gated behind the `scene` feature; rendering targets and styling come from
//! the same DOM + rule machinery as parsed HTML.
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

#[cfg(feature = "net")]
mod analytics;
mod button;
mod checkbox;
// `code_snippet` calls `SyntaxHighlighting`, which is native-only (tree-sitter),
// so mirror its `not(wasm32)` gate.
#[cfg(all(
	feature = "net",
	feature = "syntax_highlighting",
	not(target_arch = "wasm32")
))]
mod code_snippet;
mod collection_edit;
mod color_scheme;
mod dynamic_form;
mod dynamic_view;
mod error_text;
mod footer;
mod form_controls;
mod head;
mod header;
mod layout;
mod preflight;
mod render_console;
mod schema_editor;
mod schema_rebuild;
mod sidebar;
#[cfg(feature = "style")]
mod stylesheet;
mod table;
/// Shared harness for the widget tests.
#[cfg(test)]
pub(crate) mod test_ext;
mod toast;
mod value_rebuild;
mod variant_select;

#[cfg(feature = "net")]
pub use analytics::*;
pub use button::*;
pub use checkbox::*;
#[cfg(all(
	feature = "net",
	feature = "syntax_highlighting",
	not(target_arch = "wasm32")
))]
pub use code_snippet::*;
pub use color_scheme::*;
pub use dynamic_form::*;
pub use dynamic_view::*;
pub use error_text::*;
pub use footer::*;
pub use form_controls::*;
pub use head::*;
pub use header::*;
pub use layout::*;
pub use preflight::*;
// only the widget is public; its style consts (`CONSOLE_*`) stay `pub(crate)`,
// reached prefixed as `render_console::CONSOLE_INFO` within the crate.
pub use render_console::RenderConsole;
pub use schema_editor::*;
pub use schema_rebuild::*;
pub use sidebar::*;
#[cfg(feature = "style")]
pub use stylesheet::*;
pub use table::*;
pub use toast::*;
pub use value_rebuild::*;
// `button::Button` collides with the bevy_ui `Button` that leaks in via the
// `beet_core::prelude` glob below (under `bevy_default`); the explicit re-export pins
// the public `Button`, and downstream `prelude::Button`, to this crate's widget.
pub use button::Button;

use crate::prelude::RuleSet;
use beet_core::prelude::*;

/// Registers the widget set by short type path, so a name-resolved tag (eg a
/// BSX `<Head/>` or a serialized scene) builds the widget. Added by
/// [`BsxDefaultsPlugin`](crate::prelude::BsxDefaultsPlugin).
pub(crate) fn widget_plugin(app: &mut App) {
	// `button::Button` is qualified so it resolves to this crate's widget rather
	// than the bevy_ui `Button` that leaks through `beet_core::prelude` when
	// `bevy_default` is co-enabled.
	app.register_template::<button::Button>()
		.register_template::<IconButton>()
		.register_template::<Link>()
		.register_template::<ColorSchemeScript>()
		.register_template::<ErrorText>()
		.register_template::<Footer>()
		.register_template::<TextField>()
		.register_template::<TextArea>()
		.register_template::<NumberField>()
		.register_template::<Checkbox>()
		.register_template::<Select>()
		.register_template::<Form>()
		.register_template::<DynamicForm>()
		.register_template::<DynamicView>()
		.register_template::<SchemaEditor>()
		.register_template::<ToggleSchemaEditor>()
		.register_template::<Head>()
		.register_template::<Header>()
		.register_template::<HtmlDocument>()
		.register_template::<PageLayout>()
		.register_template::<PageBreak>()
		.register_template::<ContentLayout>()
		.register_template::<Preflight>()
		.register_template::<Reset>()
		.register_template::<RenderConsole>()
		.register_template::<Sidebar>()
		.register_template::<SidebarScript>()
		.register_template::<MenuButton>()
		.register_template::<Table>();
	// a schema-driven widget regenerates its subtree when the schema it renders
	// changes, so a committed schema edit reaches every form and view of it —
	// and a widget reading its document's schema generates itself when the
	// document arrives.
	app.add_systems(
		Update,
		schema_rebuild::rebuild_schema_widgets
			.run_if(schema_rebuild::schema_widgets_may_rebuild),
	);
	// ...and the value-driven twin, for the controls a schema alone does not
	// decide: a list's rows, a map's entries, an enum's payload.
	app.add_systems(Update, value_rebuild::rebuild_value_widgets);
	// a schema editor's draft forks the document its `DocRef` names, a relation
	// derived from the tree rather than authored twice.
	app.add_systems(Update, schema_editor::link_schema_drafts);
	// register the `RenderConsole` rules into the global rule set at build time, so
	// `<Stylesheet>` emits them without coupling a generic widget to the material
	// `classes` module (the line classes are set by `render_console.js`).
	app.world_mut()
		.get_resource_or_init::<RuleSet>()
		.extend_rules(render_console::console_rules());
	#[cfg(feature = "net")]
	app.register_template::<Analytics>();
	#[cfg(all(
		feature = "net",
		feature = "syntax_highlighting",
		not(target_arch = "wasm32")
	))]
	app.register_template::<CodeSnippet>();
	#[cfg(feature = "style")]
	app.register_template::<Stylesheet>();
}
