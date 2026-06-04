//! Unit tests for the ported `beet_design` widgets in `beet_ui::widgets`.
//!
//! Each test spawns the widget into a minimal scene world and asserts the
//! shape of the produced entity tree — root tag, marker component, semantic
//! classes, attribute presence. Renderer tests live with the renderer.
//!
//! Gated behind `feature = "scene"` (matching `Cargo.toml`'s
//! `required-features`) so rust-analyzer doesn't flag missing `*Props` types
//! when checking with the default feature set.
//!
//! `use beet_ui::*;` is needed so the scene `rsx!` macro's expansion of
//! `use crate::prelude::*;` resolves — integration tests are their own crate,
//! so `crate::prelude` only exists if `prelude` is brought into scope at the
//! test crate's root.
#![cfg(feature = "scene")]
beet_core::test_main!();

use beet_core::prelude::*;
use beet_ui::prelude::Button;
use beet_ui::prelude::classes;
use beet_ui::prelude::*;
use beet_ui::*;

/// A scene world with a [`PackageConfig`] resource. The document-shell widgets
/// read this synchronously at scene build via `#[scene(system)]`.
fn shell_world() -> World {
	let mut world = scene_ext::test_world();
	world.insert_resource(PackageConfig {
		title: "Beet UI".into(),
		binary_name: "beet_ui".into(),
		version: "0.0.0".into(),
		description: "test".into(),
		homepage: "https://example.test".into(),
		repository: None,
		stage: "dev".into(),
		service_access: ServiceAccess::Local,
	});
	world
}

#[beet_core::test]
fn head_emits_charset_meta() {
	let mut world = shell_world();
	let root = world.spawn_scene(rsx! { <Head/> }).unwrap().id();
	world.entity(root).get::<Head>().unwrap();

	world.with_state::<ElementQuery, _>(|query| {
		query.get(root).unwrap().tag().xpect_eq("head");
		query
			.iter_descendants_inclusive(root)
			.find(|el| el.attribute("charset").is_some())
			.unwrap()
			.attribute_string("charset")
			.xpect_eq("UTF-8");
	});
}

#[beet_core::test]
fn head_title_override_beats_package_config() {
	// per-page `ArticleMeta` title/description override the `PackageConfig`
	// defaults; here the title prop wins over the resource's "Beet UI".
	let mut world = shell_world();
	let root = world
		.spawn_scene(rsx! { <Head title="Override Title"/> })
		.unwrap()
		.id();
	world.with_state::<ElementQuery, _>(|query| {
		let values: Vec<String> = query
			.iter_descendant_values(root)
			.filter_map(|v| v.as_str().ok().map(String::from))
			.collect();
		values.iter().any(|v| v == "Override Title").xpect_true();
		values.iter().any(|v| v == "Beet UI").xpect_false();
	});
}

#[beet_core::test]
fn head_includes_pwa_meta_beyond_twelve_children() {
	// the PWA/Twitter block pushes Head past 12 children; chunking keeps them
	let mut world = shell_world();
	let root = world.spawn_scene(rsx! { <Head/> }).unwrap().id();
	world.with_state::<ElementQuery, _>(|query| {
		let names: Vec<String> = query
			.iter_descendants_inclusive(root)
			.filter(|el| el.attribute("name").is_some())
			.map(|el| el.attribute_string("name"))
			.collect();
		names
			.iter()
			.any(|n| n == "apple-mobile-web-app-capable")
			.xpect_true();
		names.iter().any(|n| n == "twitter:card").xpect_true();
	});
}

#[beet_core::test]
fn header_renders_title_from_package_config() {
	let mut world = shell_world();
	let root = world.spawn_scene(rsx! { <Header/> }).unwrap().id();

	world.with_state::<ElementQuery, _>(|query| {
		query.get(root).unwrap().tag().xpect_eq("header");
		query
			.iter_descendant_values(root)
			.any(|v| v.as_str().ok() == Some("Beet UI"))
			.xpect_true();
	});
}

#[beet_core::test]
fn footer_includes_version() {
	let mut world = shell_world();
	let root = world.spawn_scene(rsx! { <Footer/> }).unwrap().id();

	world.with_state::<ElementQuery, _>(|query| {
		query.get(root).unwrap().tag().xpect_eq("footer");
		query
			.iter_descendant_values(root)
			.filter_map(|v| v.as_str().ok())
			.any(|s| s.contains("v0.0.0"))
			.xpect_true();
	});
}

/// Render `root` to an HTML string (shell widgets emit `<head>` etc).
fn render_html(world: &mut World, root: Entity) -> String {
	HtmlRenderer::new()
		.render(&mut RenderContext::new(root, world))
		.unwrap()
		.to_string()
}

#[beet_core::test]
fn header_places_children_and_nav() {
	// a `#[scene(system)]` widget receiving children: default content sits after
	// the title, `slot="nav"` content fills the <nav>. Proves the cloned-props
	// path carries `SceneProp`s (the old slot system clobbered system widgets).
	let mut world = shell_world();
	let root = world
		.spawn_scene(rsx! {
			<Header>
				<span slot="nav">"NavLink"</span>
				"HeaderExtra"
			</Header>
		})
		.unwrap()
		.id();
	render_html(&mut world, root)
		.as_str()
		.xpect_contains("HeaderExtra")
		.xpect_contains(
			"<nav class=\"app-bar-nav\"><span>NavLink</span></nav>",
		);
}

#[beet_core::test]
fn page_layout_forwards_through_nested_composition() {
	// the headline fix: PageLayout forwards head/header_nav/body through
	// HtmlDocument into Head/Header — multi-level forwarding the old `<slot>`
	// system could not do.
	let mut world = shell_world();
	let root = world
		.spawn_scene(rsx! {
			<PageLayout>
				<meta slot="head" name="custom" content="x"/>
				<a slot="header_nav" href="/docs">"Docs"</a>
				"PageBody"
			</PageLayout>
		})
		.unwrap()
		.id();
	render_html(&mut world, root)
		.as_str()
		// the custom meta forwarded into <head> (Head's children), the nav link
		// into <nav> (Header's nav), and the body between header and footer.
		.xpect_contains("name=\"custom\"")
		.xpect_contains(">Docs</a>")
		.xpect_contains("PageBody");
}

#[beet_core::test]
fn html_document_root_is_html() {
	let mut world = shell_world();
	let root = world.spawn_scene(rsx! { <HtmlDocument/> }).unwrap().id();
	world
		.entity(root)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("html");
	world.entity(root).get::<HtmlDocument>().unwrap();
}

#[beet_core::test]
fn page_layout_root_is_html() {
	let mut world = shell_world();
	let root = world.spawn_scene(rsx! { <PageLayout/> }).unwrap().id();
	// PageLayout wraps HtmlDocument, whose root is <html>
	world
		.entity(root)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("html");
	world.entity(root).get::<PageLayout>().unwrap();
}

#[beet_core::test]
fn content_layout_root_is_html() {
	let mut world = shell_world();
	let root = world.spawn_scene(rsx! { <ContentLayout/> }).unwrap().id();
	world
		.entity(root)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("html");
	world.entity(root).get::<ContentLayout>().unwrap();
}

#[beet_core::test]
fn text_field_uses_input_classes() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <TextField name="username"/> })
		.unwrap()
		.id();
	world.entity(root).get::<TextField>().unwrap();

	world.with_state::<ElementQuery, _>(|query| {
		let view = query.get(root).unwrap();
		view.tag().xpect_eq("input");
		view.contains_class_name(&classes::INPUT).xpect_true();
		view.contains_class_name(&classes::INPUT_OUTLINED)
			.xpect_true();
	});
}

#[beet_core::test]
fn text_field_variant_changes_class() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! {
			<TextField name="x" variant=TextFieldVariant::Filled/>
		})
		.unwrap()
		.id();
	world.with_state::<ElementQuery, _>(|query| {
		let view = query.get(root).unwrap();
		view.contains_class_name(&classes::INPUT_FILLED)
			.xpect_true();
		view.contains_class_name(&classes::INPUT_OUTLINED)
			.xpect_false();
	});
}

#[beet_core::test]
fn text_field_field_attaches_field_ref() {
	let mut world = scene_ext::test_world();
	// supplied: the FieldRef component attaches to the input entity
	let root = world
		.spawn_scene(rsx! {
			<TextField name="email" field=FieldRef::new("email")/>
		})
		.unwrap()
		.id();
	world.entity(root).get::<FieldRef>().unwrap();

	// omitted: no FieldRef
	let bare = world
		.spawn_scene(rsx! { <TextField name="email"/> })
		.unwrap()
		.id();
	world.entity(bare).get::<FieldRef>().is_none().xpect_true();
}

#[beet_core::test]
fn text_field_omits_unset_optional_attrs() {
	let mut world = scene_ext::test_world();
	// omitted: no `name`/`placeholder` attributes (not an empty `name=""`)
	let bare = world.spawn_scene(rsx! { <TextField/> }).unwrap().id();
	// supplied: the attribute is present with its value
	let named = world
		.spawn_scene(rsx! { <TextField name="email"/> })
		.unwrap()
		.id();
	world.with_state::<ElementQuery, _>(|query| {
		let bare = query.get(bare).unwrap();
		bare.attribute("name").is_none().xpect_true();
		bare.attribute("placeholder").is_none().xpect_true();
		query
			.get(named)
			.unwrap()
			.attribute_string("name")
			.xpect_eq("email");
	});
}

#[beet_core::test]
fn text_area_root_is_textarea() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <TextArea name="bio"/> })
		.unwrap()
		.id();
	world
		.entity(root)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("textarea");
}

#[beet_core::test]
fn select_root_is_select() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <Select name="country"/> })
		.unwrap()
		.id();
	world.with_state::<ElementQuery, _>(|query| {
		let view = query.get(root).unwrap();
		view.tag().xpect_eq("select");
		view.contains_class_name(&classes::SELECT).xpect_true();
		view.contains_class_name(&classes::SELECT_OUTLINED)
			.xpect_true();
	});
}

#[beet_core::test]
fn form_root_is_form() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <Form name="signup"/> })
		.unwrap()
		.id();
	world
		.entity(root)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("form");
	world.entity(root).get::<Form>().unwrap();
}

#[beet_core::test]
fn error_text_carries_class() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <ErrorText message="oops"/> })
		.unwrap()
		.id();
	world.with_state::<ElementQuery, _>(|query| {
		let view = query.get(root).unwrap();
		view.tag().xpect_eq("span");
		view.contains_class_name(&classes::ERROR_TEXT).xpect_true();
		query
			.iter_descendant_values(root)
			.any(|v| v.as_str().ok() == Some("oops"))
			.xpect_true();
	});
}

#[beet_core::test]
fn table_has_head_body_foot_sections() {
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx! { <Table/> }).unwrap().id();

	world.with_state::<ElementQuery, _>(|query| {
		query.get(root).unwrap().tag().xpect_eq("table");
		let tags: Vec<_> = query
			.iter_descendants_inclusive(root)
			.map(|el| el.tag().to_string())
			.collect();
		tags.contains(&"thead".to_string()).xpect_true();
		tags.contains(&"tbody".to_string()).xpect_true();
		tags.contains(&"tfoot".to_string()).xpect_true();
	});
}

#[beet_core::test]
fn sidebar_renders_nav() {
	let mut world = scene_ext::test_world();
	let nodes = vec![SidebarNode {
		display_name: "Home".into(),
		path: Some(SmolPath::new("/")),
		..default()
	}];
	let root = world
		.spawn_scene(rsx! { <Sidebar nodes=nodes/> })
		.unwrap()
		.id();
	world.with_state::<ElementQuery, _>(|query| {
		let view = query.get(root).unwrap();
		view.tag().xpect_eq("nav");
		view.contains_class_name(&classes::SIDEBAR).xpect_true();
	});
}

#[beet_core::test]
fn sidebar_branch_renders_details() {
	let mut world = scene_ext::test_world();
	let nodes = vec![SidebarNode {
		display_name: "Docs".into(),
		path: None,
		children: vec![SidebarNode {
			display_name: "Intro".into(),
			path: Some(SmolPath::new("docs/intro")),
			..default()
		}],
		expanded: true,
		..default()
	}];
	let root = world
		.spawn_scene(rsx! { <Sidebar nodes=nodes/> })
		.unwrap()
		.id();

	world.with_state::<ElementQuery, _>(|query| {
		let tags: Vec<_> = query
			.iter_descendants_inclusive(root)
			.map(|el| el.tag().to_string())
			.collect();
		tags.contains(&"details".to_string()).xpect_true();
		tags.contains(&"summary".to_string()).xpect_true();
	});
}

#[beet_core::test]
fn sidebar_active_leaf_marks_aria_current() {
	let mut world = scene_ext::test_world();
	let nodes = vec![SidebarNode {
		display_name: "About".into(),
		path: Some(SmolPath::new("about")),
		active: true,
		..default()
	}];
	let root = world
		.spawn_scene(rsx! { <Sidebar nodes=nodes/> })
		.unwrap()
		.id();
	// the active leaf carries an `aria-current` attribute
	world.with_state::<ElementQuery, _>(|query| {
		query
			.iter_descendants_inclusive(root)
			.filter_map(|el| {
				el.attribute("aria-current").map(|a| a.value.clone())
			})
			.any(|value| value == Value::str("page"))
			.xpect_true();
	});
}

#[beet_core::test]
fn preflight_emits_style() {
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx! { <Preflight/> }).unwrap().id();
	world
		.entity(root)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("style");
	world.with_state::<ElementQuery, _>(|query| {
		query
			.iter_descendant_values(root)
			.filter_map(|v| v.as_str().ok())
			.any(|s| s.contains("box-sizing: border-box"))
			.xpect_true();
	});
}

#[beet_core::test]
fn color_scheme_script_emits_scheme_classes() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <ColorSchemeScript/> })
		.unwrap()
		.id();
	world
		.entity(root)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("script");
	world.with_state::<ElementQuery, _>(|query| {
		let body = query
			.iter_descendant_values(root)
			.filter_map(|v| v.as_str().ok())
			.collect::<String>();
		// the seed script references both scheme classes and the persistence key
		body.as_str()
			.xpect_contains(&*classes::LIGHT_SCHEME.as_selector())
			.xpect_contains(&*classes::DARK_SCHEME.as_selector())
			.xpect_contains("prefers-color-scheme")
			.xpect_contains("setColorScheme");
	});
}

#[cfg(feature = "net")]
#[beet_core::test]
fn analytics_emits_script() {
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx! { <Analytics/> }).unwrap().id();
	world
		.entity(root)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("script");
}

#[beet_core::test]
fn page_break_emits_page_break_class() {
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx! { <PageBreak/> }).unwrap().id();
	world.with_state::<ElementQuery, _>(|query| {
		query
			.get(root)
			.unwrap()
			.contains_class_name(&classes::PAGE_BREAK)
			.xpect_true();
	});
}

#[beet_core::test]
fn button_emits_base_and_variant_class() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(
			rsx! { <Button label="Save" variant=ButtonVariant::Error/> },
		)
		.unwrap()
		.id();
	world.entity(root).get::<Button>().unwrap();
	world.with_state::<ElementQuery, _>(|query| {
		let view = query.get(root).unwrap();
		view.tag().xpect_eq("button");
		view.contains_class_name(&classes::BTN).xpect_true();
		view.contains_class_name(&classes::BTN_ERROR).xpect_true();
		view.contains_class_name(&classes::BTN_FILLED).xpect_false();
		query
			.iter_descendant_values(root)
			.any(|v| v.as_str().ok() == Some("Save"))
			.xpect_true();
	});
}

#[beet_core::test]
fn icon_button_adds_icon_class() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <IconButton label="+"/> })
		.unwrap()
		.id();
	world.with_state::<ElementQuery, _>(|query| {
		let view = query.get(root).unwrap();
		view.tag().xpect_eq("button");
		view.contains_class_name(&classes::BTN_ICON).xpect_true();
	});
}

#[beet_core::test]
fn link_is_anchor_styled_as_button() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! {
			<Link label="Home" href="/" variant=ButtonVariant::Outlined/>
		})
		.unwrap()
		.id();
	world.entity(root).get::<Link>().unwrap();
	world.with_state::<ElementQuery, _>(|query| {
		let view = query.get(root).unwrap();
		view.tag().xpect_eq("a");
		view.attribute_string("href").xpect_eq("/");
		view.contains_class_name(&classes::BTN).xpect_true();
		view.contains_class_name(&classes::BTN_OUTLINED)
			.xpect_true();
	});
}
