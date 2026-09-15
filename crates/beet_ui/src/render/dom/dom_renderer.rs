//! [`DomRenderer`]: the visitor that paints a tree into the document and
//! binds every entity to what it painted.
use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::query::QueryState;
use bevy::ecs::system::SystemState;
use wasm_bindgen::JsCast;

/// The namespace an html element is created in, which needs no naming.
const XHTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";
const MATHML_NAMESPACE: &str = "http://www.w3.org/1998/Math/MathML";

/// Paints an entity tree into a DOM element: the browser's [`HtmlRenderer`],
/// one walk creating the node each entity paints as and binding it through
/// [`DomNode`].
///
/// The target stands in for the page's `<body>`: a `<body>` in the tree binds
/// to it rather than creating one, an `<html>` is transparent, and `<head>`,
/// `<script>` and `<style>` are skipped, since the head is the served page's
/// and a script has already run. Everything else is created as the string
/// sink would write it: an element with its attributes in order and its
/// merged `class`, a control with its value, a text node, a functional `bx`
/// comment. The two sinks agree byte for byte, which the parity probe pins.
pub struct DomRenderer {
	target: web_sys::Element,
}

impl DomRenderer {
	/// A renderer painting into `target`.
	pub fn new(target: web_sys::Element) -> Self { Self { target } }

	/// Paint `root`'s tree into `target`, replacing whatever it showed, and
	/// bind `root` to the target when it painted no node of its own, so a
	/// change under it reconciles against the target.
	pub fn mount(world: &mut World, root: Entity, target: web_sys::Element) {
		let mut renderer = Self::new(target);
		let roots = renderer.render(world, root);
		// one task, so the swap never shows the target empty
		while let Some(child) = renderer.target.first_child() {
			renderer.target.remove_child(&child).ok();
		}
		for node in &roots {
			renderer.target.append_child(node).ok();
		}
		if !world.entity(root).contains::<DomNode>() {
			world
				.entity_mut(root)
				.insert(DomNode::document(renderer.target));
		}
	}

	/// Paint `entity`'s subtree as fresh nodes, each bound to its entity,
	/// returning the top-level ones for the caller to place. A subtree
	/// already bound is reused, node and all, rather than painted again.
	pub fn render(
		&mut self,
		world: &mut World,
		entity: Entity,
	) -> Vec<web_sys::Node> {
		let bound = world.query::<&DomNode>();
		let mut walker = SystemState::<NodeWalker>::new(world);
		let mut painter = DomPainter::new(&self.target, world, &bound);
		walker
			.get(world)
			.expect("infallible node query")
			.walk(&mut painter, entity);
		let DomPainter {
			roots, bindings, ..
		} = painter;
		for (entity, binding) in bindings {
			world.entity_mut(entity).insert(binding);
		}
		roots
	}

	/// Tags the DOM sink never paints: the head is the served page's, a
	/// script has already run, and a style is baked into that head.
	pub(super) fn is_skipped(tag: &str) -> bool {
		matches!(tag, "head" | "script" | "style")
	}

	/// The text a value paints as: a null is empty, for a boolean attribute
	/// (present and bare) as for a control (nothing typed).
	pub(super) fn text_of(value: &Value) -> String {
		match value {
			Value::Null => String::new(),
			value => value.to_string(),
		}
	}

	/// Set a control's live value, skipped when the control already holds it
	/// so a focused control is never reassigned under its caret: `checked`
	/// for a boolean, `value` otherwise.
	pub(super) fn set_control_value(node: &web_sys::Node, value: &Value) {
		if let Some(input) = node.dyn_ref::<web_sys::HtmlInputElement>() {
			match value {
				Value::Bool(checked) => {
					if input.checked() != *checked {
						input.set_checked(*checked);
					}
				}
				value => {
					let text = Self::text_of(value);
					if input.value() != text {
						input.set_value(&text);
					}
				}
			}
		} else if let Some(area) =
			node.dyn_ref::<web_sys::HtmlTextAreaElement>()
		{
			let text = Self::text_of(value);
			if area.value() != text {
				area.set_value(&text);
			}
		} else if let Some(select) =
			node.dyn_ref::<web_sys::HtmlSelectElement>()
		{
			let text = Self::text_of(value);
			if select.value() != text {
				select.set_value(&text);
			}
		}
	}
}

/// The visitor of one [`DomRenderer::render`]: what it painted and bound,
/// handed back once the walk releases the world.
struct DomPainter<'w> {
	target: &'w web_sys::Element,
	document: web_sys::Document,
	world: &'w World,
	/// The bindings made before this walk, so a bound subtree is reused.
	bound: &'w QueryState<&'static DomNode>,
	/// The elements being painted into, innermost last; empty at the top
	/// level, where a created node is a root.
	stack: Vec<web_sys::Element>,
	/// The top-level nodes the walk produced, in order.
	roots: Vec<web_sys::Node>,
	/// The bindings the walk made, applied once it holds the world again.
	bindings: Vec<(Entity, DomNode)>,
	/// The value of the `<select>` being painted, for the `<option>` it names.
	select_value: Option<String>,
}

impl<'w> DomPainter<'w> {
	fn new(
		target: &'w web_sys::Element,
		world: &'w World,
		bound: &'w QueryState<&'static DomNode>,
	) -> Self {
		Self {
			target,
			document: target
				.owner_document()
				.unwrap_or_else(document_ext::document),
			world,
			bound,
			stack: default(),
			roots: default(),
			bindings: default(),
			select_value: None,
		}
	}

	/// Append `node` to the element being painted into, or record it as a
	/// root.
	fn place(&mut self, node: &web_sys::Node) {
		match self.stack.last() {
			Some(parent) => {
				parent.append_child(node).ok();
			}
			None => self.roots.push(node.clone()),
		}
	}

	/// Bind `entity` to the node it painted as, stamped so the node answers
	/// for it.
	fn bind(&mut self, entity: Entity, binding: DomNode) {
		DomNode::stamp(binding.get(), entity);
		self.bindings.push((entity, binding));
	}

	/// The element `tag` creates, in the namespace of the element it lands
	/// in; an unusable tag paints as a `<span>` and says so.
	fn create_element(&self, tag: &str) -> web_sys::Element {
		let parent = self.stack.last().unwrap_or(self.target);
		let namespace: Option<String> = match tag {
			"svg" => Some(SVG_NAMESPACE.into()),
			"math" => Some(MATHML_NAMESPACE.into()),
			_ => parent
				.namespace_uri()
				.filter(|namespace| namespace != XHTML_NAMESPACE),
		};
		let created = match &namespace {
			Some(namespace) => {
				self.document.create_element_ns(Some(namespace), tag)
			}
			None => self.document.create_element(tag),
		};
		created.unwrap_or_else(|err| {
			error!("`<{tag}>` cannot be created, painting a span: {err:?}");
			self.document
				.create_element("span")
				.expect("a span always creates")
		})
	}

	/// Set the element's attributes as the string sink writes them, in
	/// order, binding each attribute entity to the element it sets; the
	/// merged `class` follows the control's value, see [`Self::apply_class`].
	fn apply_attributes(
		&mut self,
		element: &web_sys::Element,
		view: &ElementView,
	) {
		for attr in &view.attributes {
			if attr.key() == "class" {
				continue;
			}
			element
				.set_attribute(attr.key(), &DomRenderer::text_of(attr.value))
				.ok();
		}
		for attr in &view.attributes {
			self.bindings
				.push((attr.entity, DomNode::attribute(element.clone())));
		}
	}

	/// The merged `class`, last as the string sink writes it, so the two
	/// serialize their attributes in one order.
	fn apply_class(&self, element: &web_sys::Element, view: &ElementView) {
		if let Some(class) = view.class_attribute() {
			element.set_attribute("class", &class).ok();
		}
	}

	/// A control's value in markup, as the string sink writes it (see
	/// [`HtmlRenderer`]), so the served page and the painted one agree: an
	/// `<input>`'s default value, a `<select>`'s marked option, a
	/// `<textarea>`'s content.
	fn apply_value(&mut self, element: &web_sys::Element, view: &ElementView) {
		match (view.tag(), view.value) {
			("input", Some(value)) if !matches!(value, Value::Bool(_)) => {
				element.set_attribute("value", &value.to_string()).ok();
			}
			("textarea", Some(value)) => {
				element.set_text_content(Some(&value.to_string()));
			}
			("select", value) => {
				self.select_value = value.map(ToString::to_string);
			}
			("option", _)
				if self.select_value.as_deref()
					== Some(view.option_value().as_str()) =>
			{
				element.set_attribute("selected", "").ok();
			}
			_ => {}
		}
	}
}

impl NodeVisitor for DomPainter<'_> {
	fn skip_node(
		&mut self,
		cx: &VisitContext,
		(_, _, element, ..): &NodeView,
	) -> bool {
		// a subtree already painted is reused where it stands in the walk
		if let Ok(DomNode::Node(node)) =
			self.bound.get_manual(self.world, cx.entity)
		{
			let node = (**node).clone();
			self.place(&node);
			return true;
		}
		element.is_some_and(|element| DomRenderer::is_skipped(element.tag()))
	}

	fn visit_comment(&mut self, cx: &VisitContext, comment: &Comment) {
		// functional `bx` anchors paint; authoring comments never reach a page
		if !comment.starts_with("bx") {
			return;
		}
		let node = self.document.create_comment(comment);
		self.place(&node);
		self.bind(cx.entity, DomNode::node(node));
	}

	fn visit_element(&mut self, cx: &VisitContext, view: ElementView) {
		match view.tag() {
			// the document's own: transparent, and the target stands in
			"html" => return,
			"body" => {
				let target = self.target.clone();
				self.apply_attributes(&target, &view);
				self.apply_class(&target, &view);
				self.bind(cx.entity, DomNode::document(target));
				return;
			}
			_ => {}
		}
		let element = self.create_element(view.tag());
		self.apply_attributes(&element, &view);
		self.apply_value(&element, &view);
		self.apply_class(&element, &view);
		self.place(&element);
		self.stack.push(element.clone());
		self.bind(cx.entity, DomNode::node(element));
	}

	fn leave_element(&mut self, _cx: &VisitContext, element: &Element) {
		let painted = match element.tag() {
			"html" | "body" => None,
			_ => self.stack.pop(),
		};
		// a select's value lands once its options exist
		if element.tag() == "select"
			&& let Some(value) = self.select_value.take()
			&& let Some(select) = painted.and_then(|element| {
				element.dyn_into::<web_sys::HtmlSelectElement>().ok()
			}) {
			select.set_value(&value);
		}
	}

	fn visit_value(&mut self, cx: &VisitContext, value: &Value) {
		let node = self.document.create_text_node(&value.to_string());
		self.place(&node);
		self.bind(cx.entity, DomNode::node(node));
	}
}

#[cfg(test)]
mod test {
	use super::super::test_ext::*;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The parity probe: what the DOM sink paints serializes exactly as the
	/// string sink writes, through attributes in order, merged classes,
	/// escaped text, a control's value, a checkbox's `checked` and a select's
	/// marked option.
	#[beet_core::test(browser)]
	fn paints_what_the_string_sink_writes() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<main class="page" {Classes::new(["note"])}>
				<h1 id="title">"Garden & <friends>"</h1>
				<p data-kind="note">"Every "<em>"entity"</em>" is a scene"</p>
				<TextField field={FieldRef::new("name")} placeholder="who"/>
				<Select field={FieldRef::new("bed")}>
					<option value="carrots">"Carrots"</option>
					<option value="beets">"Beets"</option>
				</Select>
				<Checkbox field={FieldRef::new("done")}/>
				<textarea {Value::str("a\nb")}/>
			</main>
		});
		app.world_mut()
			.entity_mut(host)
			.insert(Document::new(value!({
				"name": "pete",
				"bed": "beets",
				"done": true,
			})));
		// the bindings sync, then the checkbox mirrors its `checked`
		app.update();
		app.update();
		let target = paint(&mut app, host);
		target
			.inner_html()
			.xpect_eq(ssr_html(app.world_mut(), host));
	}

	/// The probe over the scene editor page itself, editor open: the page
	/// the browser suite edits paints byte-equal to its SSR.
	#[cfg(feature = "template_serde")]
	#[beet_core::test(browser)]
	fn the_scene_editor_page_paints_byte_equal() {
		let mut app = dom_app();
		app.init_plugin::<MinimalTypesPlugin>();
		let host = app.world_mut().spawn_empty().id();
		TemplateLoader::new(app.world_mut())
			.with_entity(host)
			.load(&MediaBytes::new_bsx(
				"<main><h1>Garden</h1><p class=\"note\"></p><ToggleSceneEditor/></main>",
			))
			.unwrap();
		SceneDocument::fork(app.world_mut(), host, MediaType::Json).unwrap();
		let details = element(app.world_mut(), "details");
		app.world_mut()
			.spawn((AttributeOf::new(details), Attribute::new("open")));
		for _ in 0..8 {
			app.update();
		}
		let target = paint(&mut app, host);
		target
			.inner_html()
			.xpect_eq(ssr_html(app.world_mut(), host));
	}
}
