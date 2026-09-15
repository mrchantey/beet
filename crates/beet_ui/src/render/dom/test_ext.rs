//! Harness for the DOM sink tests: a world with the widget set, painting into
//! a container in the test page.
use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use wasm_bindgen::JsCast;

/// An app with the template substrate, the widget set, the DOM sink's
/// incremental pass and the input path (which composes the form controls and
/// the focus model).
pub(super) fn dom_app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TemplatePlugin,
		DocumentPlugin,
		BsxDefaultsPlugin,
		DomRenderPlugin,
		DomInputPlugin,
	));
	app
}

/// Build `template` under a transparent host, returning the host.
pub(super) fn build(
	app: &mut App,
	template: impl bevy::ecs::template::Template<Output = ()>,
) -> Entity {
	let host = app.world_mut().spawn_empty().id();
	let page = app.world_mut().spawn_template(template).unwrap().id();
	app.world_mut().entity_mut(page).insert(ChildOf(host));
	host
}

/// Replace the document the host's bindings resolve against.
pub(super) fn set_document(app: &mut App, host: Entity, value: Value) {
	app.world_mut()
		.entity_mut(host)
		.insert(Document::new(value));
}

/// A fresh container in the document body, so selection and focus behave as
/// on a page.
pub(super) fn container() -> web_sys::Element {
	let div = document_ext::create_div();
	document_ext::append_child(&div);
	div.into()
}

/// Paint `host` into a fresh container and run a frame, returning the
/// container.
pub(super) fn paint(app: &mut App, host: Entity) -> web_sys::Element {
	let target = container();
	DomRenderer::mount(app.world_mut(), host, target.clone());
	app.update();
	target
}

/// The html the string sink writes for `root`, as the browser serializes it:
/// the normalizing half of the parity probe, so a void tag's slash or a bare
/// boolean attribute compares by meaning rather than spelling.
pub(super) fn ssr_html(world: &mut World, root: Entity) -> String {
	let html = HtmlRenderer::new()
		.render(&mut RenderContext::new(root, world))
		.unwrap()
		.to_string();
	let div = document_ext::create_div();
	div.set_inner_html(&html);
	div.inner_html()
}

/// The first element with `tag` in document order.
pub(super) fn element(world: &mut World, tag: &str) -> Entity {
	crate::widgets::test_ext::element_in(world, tag)
}

/// The node `entity` is bound to.
pub(super) fn node_of(world: &World, entity: Entity) -> web_sys::Node {
	world
		.get::<DomNode>(entity)
		.expect("the entity is bound")
		.get()
		.clone()
}

/// The entity of `element`'s attribute named `key`.
pub(super) fn attribute_of(
	world: &World,
	element: Entity,
	key: &str,
) -> Entity {
	world
		.get::<Attributes>(element)
		.into_iter()
		.flat_map(|attributes| attributes.iter())
		.find(|attribute| {
			world
				.get::<Attribute>(*attribute)
				.is_some_and(|attribute| attribute.as_str() == key)
		})
		.unwrap_or_else(|| panic!("no `{key}` attribute"))
}

/// A bubbling `name` event that may be cancelled, so a test can see what the
/// input path prevented.
pub(super) fn bubbling(name: &str) -> web_sys::Event {
	let init = web_sys::EventInit::new();
	init.set_bubbles(true);
	init.set_cancelable(true);
	web_sys::Event::new_with_event_init_dict(name, &init).unwrap()
}

/// A bubbling, cancellable `keydown` for `key`.
pub(super) fn keydown(key: &str) -> web_sys::Event {
	let init = web_sys::KeyboardEventInit::new();
	init.set_key(key);
	init.set_bubbles(true);
	init.set_cancelable(true);
	web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
		.unwrap()
		.into()
}

/// A bubbling, cancellable `submit`, by `submitter` when a button submitted.
pub(super) fn submit(
	submitter: Option<&web_sys::HtmlElement>,
) -> web_sys::Event {
	let init = web_sys::SubmitEventInit::new();
	init.set_bubbles(true);
	init.set_cancelable(true);
	init.set_submitter(submitter);
	web_sys::SubmitEvent::new_with_event_init_dict("submit", &init)
		.unwrap()
		.into()
}

/// Dispatch `ev` on `node`, returning whether its default survived.
pub(super) fn fire(node: &web_sys::Node, ev: &web_sys::Event) -> bool {
	node.dispatch_event(ev).unwrap()
}

/// The element `entity` paints as.
pub(super) fn html_element(
	world: &World,
	entity: Entity,
) -> web_sys::HtmlElement {
	node_of(world, entity)
		.dyn_into::<web_sys::HtmlElement>()
		.expect("the entity paints as an html element")
}

/// The field at `path` of the document on `host`.
pub(super) fn field(world: &World, host: Entity, path: &str) -> Value {
	world
		.get::<Document>(host)
		.expect("the host holds the document")
		.get_field::<Value>(&FieldPath::parse(path))
		.unwrap_or_default()
}
