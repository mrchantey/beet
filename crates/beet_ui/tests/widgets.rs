//! Unit tests for the ported `beet_design` widgets in `beet_ui::widgets`.
//!
//! Each test spawns the widget into a minimal scene world and asserts the
//! shape of the produced entity tree — root tag, marker component, semantic
//! classes, attribute presence. Renderer tests live with the renderer.
beet_core::test_main!();

use beet_core::prelude::*;
use beet_ui::prelude::*;
use beet_ui::*;

/// Setup: a scene world with a [`PackageConfig`] resource. The document-shell
/// widgets read this synchronously at scene build via `#[scene(system)]`.
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

	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("head");
	world.entity(root).get::<Head>().unwrap();

	// charset attr lives on one of the child meta entities
	let children = world.entity(root).get::<Children>().unwrap();
	let mut saw_charset = false;
	for child in children.iter() {
		if let Some(attrs) = world.entity(child).get::<Attributes>() {
			for attr_entity in attrs.iter() {
				let attr_ref = world.entity(attr_entity);
				if let (Some(attr), Some(value)) =
					(attr_ref.get::<Attribute>(), attr_ref.get::<Value>())
				{
					if **attr == "charset" {
						value.as_str().unwrap().xpect_eq("UTF-8");
						saw_charset = true;
					}
				}
			}
		}
	}
	saw_charset.xpect_true();
}

#[beet_core::test]
fn header_renders_title_from_package_config() {
	let mut world = shell_world();
	let root = world.spawn_scene(rsx! { <Header/> }).unwrap().id();

	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("header");

	// the <a> child contains the package title as text
	let children = world.entity(root).get::<Children>().unwrap();
	let mut saw_title = false;
	for child in children.iter() {
		if let Some(el) = world.entity(child).get::<Element>() {
			if el.tag() == "a" {
				if let Some(grandchildren) =
					world.entity(child).get::<Children>()
				{
					for gc in grandchildren.iter() {
						if let Some(v) = world.entity(gc).get::<Value>() {
							if v.as_str().ok() == Some("Beet UI") {
								saw_title = true;
							}
						}
					}
				}
			}
		}
	}
	saw_title.xpect_true();
}

#[beet_core::test]
fn footer_includes_version() {
	let mut world = shell_world();
	let root = world.spawn_scene(rsx! { <Footer/> }).unwrap().id();

	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("footer");

	// at least one descendant value contains "v0.0.0"
	fn descendant_values(world: &World, entity: Entity, out: &mut Vec<String>) {
		if let Some(v) = world.entity(entity).get::<Value>() {
			if let Ok(s) = v.as_str() {
				out.push(s.to_string());
			}
		}
		if let Some(children) = world.entity(entity).get::<Children>() {
			for child in children.iter() {
				descendant_values(world, child, out);
			}
		}
	}
	let mut values = Vec::new();
	descendant_values(&world, root, &mut values);
	values.iter().any(|s| s.contains("v0.0.0")).xpect_true();
}

#[beet_core::test]
fn document_layout_root_is_html() {
	let mut world = shell_world();
	let root = world.spawn_scene(rsx! { <DocumentLayout/> }).unwrap().id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("html");
	world.entity(root).get::<DocumentLayout>().unwrap();
}

#[beet_core::test]
fn page_layout_root_is_html() {
	let mut world = shell_world();
	let root = world.spawn_scene(rsx! { <PageLayout/> }).unwrap().id();
	// PageLayout wraps DocumentLayout, whose root is <html>
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("html");
	world.entity(root).get::<PageLayout>().unwrap();
}

#[beet_core::test]
fn content_layout_root_is_html() {
	let mut world = shell_world();
	let root = world.spawn_scene(rsx! { <ContentLayout/> }).unwrap().id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("html");
	world.entity(root).get::<ContentLayout>().unwrap();
}

#[beet_core::test]
fn text_field_uses_input_classes() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <TextField name="username"/> })
		.unwrap()
		.id();

	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("input");
	world.entity(root).get::<TextField>().unwrap();
	let classes = world.entity(root).get::<Classes>().unwrap();
	classes.contains_selector("input").xpect_true();
	classes.contains_selector("input-outlined").xpect_true();
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
	let classes = world.entity(root).get::<Classes>().unwrap();
	classes.contains_selector("input-filled").xpect_true();
	classes.contains_selector("input-outlined").xpect_false();
}

#[beet_core::test]
fn text_area_root_is_textarea() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <TextArea name="bio"/> })
		.unwrap()
		.id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("textarea");
}

#[beet_core::test]
fn select_root_is_select() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <Select name="country"/> })
		.unwrap()
		.id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("select");
	let classes = world.entity(root).get::<Classes>().unwrap();
	classes.contains_selector("select").xpect_true();
	classes.contains_selector("select-outlined").xpect_true();
}

#[beet_core::test]
fn form_root_is_form() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <Form name="signup"/> })
		.unwrap()
		.id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("form");
	world.entity(root).get::<Form>().unwrap();
}

#[beet_core::test]
fn error_text_carries_class() {
	let mut world = scene_ext::test_world();
	let root = world
		.spawn_scene(rsx! { <ErrorText message="oops"/> })
		.unwrap()
		.id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("span");
	let classes = world.entity(root).get::<Classes>().unwrap();
	classes.contains_selector("error-text").xpect_true();

	let children = world.entity(root).get::<Children>().unwrap();
	children.len().xpect_eq(1);
	world
		.entity(children[0])
		.get::<Value>()
		.unwrap()
		.xpect_eq(Value::new("oops"));
}

#[beet_core::test]
fn table_has_head_body_foot_sections() {
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx! { <Table/> }).unwrap().id();

	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("table");
	let children = world.entity(root).get::<Children>().unwrap();
	let tags: Vec<_> = children
		.iter()
		.filter_map(|c| world.entity(c).get::<Element>())
		.map(|e| e.tag().to_string())
		.collect();
	tags.contains(&"thead".to_string()).xpect_true();
	tags.contains(&"tbody".to_string()).xpect_true();
	tags.contains(&"tfoot".to_string()).xpect_true();
}

#[beet_core::test]
fn sidebar_renders_nav() {
	let mut world = scene_ext::test_world();
	let nodes = vec![SidebarNode {
		display_name: "Home".into(),
		path: Some("/".into()),
		children: vec![],
		expanded: false,
	}];
	let root = world
		.spawn_scene(rsx! { <Sidebar nodes=nodes/> })
		.unwrap()
		.id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("nav");
	let classes = world.entity(root).get::<Classes>().unwrap();
	classes.contains_selector("sidebar").xpect_true();
}

#[beet_core::test]
fn sidebar_branch_renders_details() {
	let mut world = scene_ext::test_world();
	let nodes = vec![SidebarNode {
		display_name: "Docs".into(),
		path: None,
		children: vec![SidebarNode {
			display_name: "Intro".into(),
			path: Some("/docs/intro".into()),
			children: vec![],
			expanded: false,
		}],
		expanded: true,
	}];
	let root = world
		.spawn_scene(rsx! { <Sidebar nodes=nodes/> })
		.unwrap()
		.id();

	// walk descendants looking for <details>
	fn descendant_tags(world: &World, entity: Entity, out: &mut Vec<String>) {
		if let Some(el) = world.entity(entity).get::<Element>() {
			out.push(el.tag().to_string());
		}
		if let Some(children) = world.entity(entity).get::<Children>() {
			for c in children.iter() {
				descendant_tags(world, c, out);
			}
		}
	}
	let mut tags = Vec::new();
	descendant_tags(&world, root, &mut tags);
	tags.contains(&"details".to_string()).xpect_true();
	tags.contains(&"summary".to_string()).xpect_true();
}

#[cfg(feature = "net")]
#[beet_core::test]
fn analytics_emits_script() {
	let mut world = scene_ext::test_world();
	let root = world.spawn_scene(rsx! { <Analytics/> }).unwrap().id();
	world.entity(root).get::<Element>().unwrap().tag().xpect_eq("script");
}
