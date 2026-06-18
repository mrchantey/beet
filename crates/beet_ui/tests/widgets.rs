//! Unit tests for the `beet_ui::widgets`.
//!
//! Each test spawns a widget into a minimal template world and asserts the
//! shape of the produced entity tree — root tag, semantic classes, attribute
//! presence, slotted content. Renderer tests live with the renderer.
//!
//! Gated behind `feature = "template"` (matching `Cargo.toml`'s
//! `required-features`).
#![cfg(feature = "template")]
beet_core::test_main!();

use beet_core::prelude::*;
use beet_ui::prelude::classes;
use beet_ui::prelude::*;

/// A scene world with a [`PackageConfig`] resource. The document-layout widgets
/// read this synchronously at template build via `#[template(system)]`.
fn layout_world() -> World {
	let mut world = ui_world();
	world.insert_resource(PackageConfig {
		title: "Beet UI".into(),
		description: "test".into(),
		binary_name: Some("beet_ui".into()),
		version: "0.0.0".into(),
		homepage: Some("https://example.test".into()),
		repository: None,
		stage: "dev".into(),
		service_access: ServiceAccess::Local,
	});
	world
}

#[beet_core::test]
fn head_emits_charset_meta() {
	let mut world = layout_world();
	let root = world.spawn_template(rsx! { <Head/> }).unwrap().id();

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
fn head_title_defaults_to_package_config() {
	// the standalone head renders its own `<title>` seeded from the package title.
	let mut world = layout_world();
	let root = world.spawn_template(rsx! { <Head/> }).unwrap().id();
	let html = render_html(&mut world, root);
	html.as_str().xpect_contains("<title>Beet UI</title>");
}

#[beet_core::test]
fn head_omit_title_drops_own_title() {
	// `omit_title` lets a layout own the single `<title>`: the head renders none.
	let mut world = layout_world();
	let root = world
		.spawn_template(rsx! { <Head omit_title=true/> })
		.unwrap()
		.id();
	let html = render_html(&mut world, root);
	html.matches("<title>").count().xpect_eq(0);
}

#[beet_core::test]
fn head_includes_pwa_meta_beyond_twelve_children() {
	// the PWA/Twitter block pushes Head past 12 children; chunking keeps them
	let mut world = layout_world();
	let root = world.spawn_template(rsx! { <Head/> }).unwrap().id();
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
fn head_emits_single_og_site_name_from_package_config() {
	// the default head owns og:site_name, bound to `PackageConfig.title`; the
	// seeded value renders the title even before any document sync.
	let mut world = layout_world();
	let root = world.spawn_template(rsx! { <Head/> }).unwrap().id();
	let html = render_html(&mut world, root);
	// exactly one tag, carrying the resource title
	html.matches("og:site_name").count().xpect_eq(1);
	html.as_str()
		.xpect_contains("property=\"og:site_name\" content=\"Beet UI\"");
}

#[beet_core::test]
fn header_renders_title_from_package_config() {
	let mut world = layout_world();
	let root = world.spawn_template(rsx! { <Header/> }).unwrap().id();

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
	let mut world = layout_world();
	let root = world.spawn_template(rsx! { <Footer/> }).unwrap().id();

	world.with_state::<ElementQuery, _>(|query| {
		query.get(root).unwrap().tag().xpect_eq("footer");
		query
			.iter_descendant_values(root)
			.filter_map(|v| v.as_str().ok())
			.any(|s| s.contains("v0.0.0"))
			.xpect_true();
	});
}

/// Render `root` to an HTML string (layout widgets emit `<head>` etc).
fn render_html(world: &mut World, root: Entity) -> String {
	HtmlRenderer::new()
		.render(&mut RenderContext::new(root, world))
		.unwrap()
		.to_string()
}

#[beet_core::test]
fn header_places_children_and_nav() {
	// a `#[template(system)]` widget receiving children: default content sits after
	// the title, `slot="nav"` content fills the <nav>. Proves the cloned-props
	// path carries slot children through a system template.
	let mut world = layout_world();
	let root = world
		.spawn_template(rsx! {
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
	let mut world = layout_world();
	let root = world
		.spawn_template(rsx! {
			<PageLayout>
				<meta slot="head" name="custom" content="x"/>
				<a slot="header-nav" href="/docs">"Docs"</a>
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
	let mut world = layout_world();
	let root = world.spawn_template(rsx! { <HtmlDocument/> }).unwrap().id();
	world
		.entity(root)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("html");
}

#[beet_core::test]
fn page_layout_root_is_html() {
	let mut world = layout_world();
	let root = world.spawn_template(rsx! { <PageLayout/> }).unwrap().id();
	// PageLayout wraps HtmlDocument, whose root is <html>
	world
		.entity(root)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("html");
}

#[beet_core::test]
fn content_layout_root_is_html() {
	let mut world = layout_world();
	let root = world
		.spawn_template(rsx! { <ContentLayout/> })
		.unwrap()
		.id();
	world
		.entity(root)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("html");
}

#[beet_core::test]
fn text_field_uses_input_classes() {
	let mut world = ui_world();
	let root = world
		.spawn_template(rsx! { <TextField name="username"/> })
		.unwrap()
		.id();

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
	let mut world = ui_world();
	let root = world
		.spawn_template(rsx! {
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
	let mut world = ui_world();
	// supplied: the FieldRef component attaches to the input entity
	let root = world
		.spawn_template(rsx! {
			<TextField name="email" field=FieldRef::new("email")/>
		})
		.unwrap()
		.id();
	world.entity(root).get::<FieldRef>().unwrap();

	// omitted: no FieldRef
	let bare = world
		.spawn_template(rsx! { <TextField name="email"/> })
		.unwrap()
		.id();
	world.entity(bare).get::<FieldRef>().is_none().xpect_true();
}

#[beet_core::test]
fn text_field_omits_unset_optional_attrs() {
	let mut world = ui_world();
	// omitted: no `name`/`placeholder` attributes (not an empty `name=""`)
	let bare = world.spawn_template(rsx! { <TextField/> }).unwrap().id();
	// supplied: the attribute is present with its value
	let named = world
		.spawn_template(rsx! { <TextField name="email"/> })
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
	let mut world = ui_world();
	let root = world
		.spawn_template(rsx! { <TextArea name="bio"/> })
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
	let mut world = ui_world();
	let root = world
		.spawn_template(rsx! { <Select name="country"/> })
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
	let mut world = ui_world();
	let root = world
		.spawn_template(rsx! { <Form name="signup"/> })
		.unwrap()
		.id();
	world
		.entity(root)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("form");
}

#[beet_core::test]
fn error_text_carries_class() {
	let mut world = ui_world();
	let root = world
		.spawn_template(rsx! { <ErrorText message="oops"/> })
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
	let mut world = ui_world();
	let root = world.spawn_template(rsx! { <Table/> }).unwrap().id();

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
	let mut world = ui_world();
	let nodes = vec![SidebarNode {
		display_name: "Home".into(),
		path: Some(SmolPath::new("/")),
		..default()
	}];
	let root = world
		.spawn_template(rsx! { <Sidebar nodes=nodes/> })
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
	let mut world = ui_world();
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
		.spawn_template(rsx! { <Sidebar nodes=nodes/> })
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
	let mut world = ui_world();
	let nodes = vec![SidebarNode {
		display_name: "About".into(),
		path: Some(SmolPath::new("about")),
		active: true,
		..default()
	}];
	let root = world
		.spawn_template(rsx! { <Sidebar nodes=nodes/> })
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
	let mut world = ui_world();
	let root = world.spawn_template(rsx! { <Preflight/> }).unwrap().id();
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
	let mut world = ui_world();
	let root = world
		.spawn_template(rsx! { <ColorSchemeScript/> })
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
	let mut world = ui_world();
	let root = world.spawn_template(rsx! { <Analytics/> }).unwrap().id();
	world
		.entity(root)
		.get::<Element>()
		.unwrap()
		.tag()
		.xpect_eq("script");
}

#[beet_core::test]
fn page_break_emits_page_break_class() {
	let mut world = ui_world();
	let root = world.spawn_template(rsx! { <PageBreak/> }).unwrap().id();
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
	let mut world = ui_world();
	let root = world
		.spawn_template(
			rsx! { <Button variant=ButtonVariant::Error>"Save"</Button> },
		)
		.unwrap()
		.id();
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
	let mut world = ui_world();
	let root = world
		.spawn_template(rsx! { <IconButton>"+"</IconButton> })
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
	let mut world = ui_world();
	let root = world
		.spawn_template(rsx! {
			<Link href="/" variant=ButtonVariant::Outlined>"Home"</Link>
		})
		.unwrap()
		.id();
	world.with_state::<ElementQuery, _>(|query| {
		let view = query.get(root).unwrap();
		view.tag().xpect_eq("a");
		view.attribute_string("href").xpect_eq("/");
		view.contains_class_name(&classes::BTN).xpect_true();
		view.contains_class_name(&classes::BTN_OUTLINED)
			.xpect_true();
	});
}
