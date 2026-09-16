//! The one central widget installation: every widget registered by short type
//! path, plus the systems and rules the widget set needs to run.
use super::browser::*;
use super::chrome::*;
use super::controls::*;
use super::debug::*;
use super::schema_ui::*;
use super::toast::ToastPlugin;
use crate::prelude::RuleSet;
use beet_core::prelude::*;

/// Registers the widget set by short type path, so a name-resolved tag (eg a
/// BSX `<Head/>` or a serialized scene) builds the widget. Added by
/// [`BsxDefaultsPlugin`](crate::prelude::BsxDefaultsPlugin).
pub(crate) fn widget_plugin(app: &mut App) {
	// `controls::button::Button` is qualified so it resolves to this crate's
	// widget rather than the bevy_ui `Button` that leaks through
	// `beet_core::prelude` when `bevy_default` is co-enabled.
	app.register_template::<super::controls::button::Button>()
		.register_template::<IconButton>()
		.register_template::<Link>()
		.register_template::<ColorSchemeScript>()
		// `Error` is qualified for the same reason as `Button`: the name is common
		// enough that a prelude glob may also define one.
		.register_template::<super::controls::error::Error>()
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
	// a baked `<Stylesheet/>` follows rules registered after it baked
	app.add_systems(
		crate::parse::PostParseTree,
		super::refresh_stylesheets.after(crate::style::ResolveStylesSet),
	);
	// a schema-driven widget regenerates its subtree when the schema it renders
	// changes, so a committed schema edit reaches every form and view of it —
	// and a widget reading its document's schema generates itself when the
	// document arrives.
	app.add_systems(
		Update,
		super::schema_ui::schema_rebuild::rebuild_schema_widgets.run_if(
			super::schema_ui::schema_rebuild::schema_widgets_may_rebuild,
		),
	);
	// ...and the value-driven twin, for the controls a schema alone does not
	// decide: a list's rows, a map's entries, an enum's payload.
	app.add_systems(
		Update,
		super::schema_ui::value_rebuild::rebuild_value_widgets,
	);
	// a schema editor commits every change of the schema document it names
	// against the data document it sits in, the one transaction an edit rides;
	// the commit evolves data off a task, so the plugin that registers it
	// declares the bridge that carries it
	app.init_plugin::<AsyncPlugin>()
		.add_systems(Update, super::schema_ui::editor::commit_schema_edits);
	// the scene editor: its tag survives a fork as a reflected component, its
	// frame binds the scene it sits in, and its tree and inspector follow the
	// scene's selection
	#[cfg(feature = "template_serde")]
	super::scene_editor::scene_editor_plugin(app);
	// `Toast` is target-neutral, so its tag registration and expiry timer belong
	// to the widget set rather than to the one renderer that pops toasts today
	// (the charcell clipboard). `init_plugin` is idempotent, so a charcell app
	// installing it directly still gets exactly one.
	app.init_plugin::<ToastPlugin>();
	// register the `RenderConsole` rules into the global rule set at build time, so
	// `<Stylesheet>` emits them without coupling a generic widget to the material
	// `classes` module (the line classes are set by `render_console.js`).
	app.world_mut()
		.get_resource_or_init::<RuleSet>()
		.extend_rules(super::debug::render_console::console_rules());
	#[cfg(feature = "net")]
	app.register_template::<Analytics>();
	#[cfg(all(
		feature = "net",
		feature = "syntax_highlighting",
		not(target_arch = "wasm32")
	))]
	app.register_template::<super::code_snippet::CodeSnippet>();
	#[cfg(all(feature = "net", feature = "mermaid"))]
	app.register_template::<super::mermaid::Mermaid>();
	#[cfg(feature = "style")]
	app.register_template::<Stylesheet>();
}
