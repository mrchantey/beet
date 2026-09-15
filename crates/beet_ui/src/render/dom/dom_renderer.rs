//! [`DomRenderer`]: the visitor that paints a tree into the document and
//! binds every entity to what it painted, adopting a served page where it
//! already agrees.
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
///
/// A mount ([`Self::mount`]) adopts rather than repaints: the walk and the
/// target's served children advance in lockstep, a served node the world
/// agrees with is bound as it stands (its listeners, its focus and its typed
/// value intact) and one the world disagrees with is replaced in place by a
/// fresh paint. The [`Adoption`] it returns is the conformance measure: a
/// served page and the world that rendered it adopt with nothing replaced.
pub struct DomRenderer {
	target: web_sys::Element,
}

/// What a mount found: the served nodes it bound as they stood, the nodes it
/// created or removed because the served page disagreed with the world, and
/// the adopted nodes whose text or attributes it had to set.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Adoption {
	/// Served nodes bound in place.
	pub adopted: usize,
	/// Nodes created or removed because the served page disagreed.
	pub replaced: usize,
	/// Adopted nodes whose text or attributes differed and were set.
	pub patched: usize,
}

impl Adoption {
	/// Whether the served page and the world agreed on every node.
	pub fn is_clean(&self) -> bool { self.replaced == 0 && self.patched == 0 }
}

impl core::fmt::Display for Adoption {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(
			f,
			"{} adopted, {} replaced, {} patched",
			self.adopted, self.replaced, self.patched
		)
	}
}

impl DomRenderer {
	/// A renderer painting into `target`.
	pub fn new(target: web_sys::Element) -> Self { Self { target } }

	/// Paint `root`'s tree into `target`, adopting the children it already
	/// shows where they agree with the world and replacing the rest in place,
	/// and bind `root` to the target when it painted no node of its own, so a
	/// change under it reconciles against the target. An empty target is a
	/// plain first paint.
	pub fn mount(
		world: &mut World,
		root: Entity,
		target: web_sys::Element,
	) -> Adoption {
		let mut renderer = Self::new(target);
		let frame = Frame::adopting(&renderer.target);
		let (_, adoption) = renderer.walk(world, root, frame);
		if !world.entity(root).contains::<DomNode>() {
			world
				.entity_mut(root)
				.insert(DomNode::document(renderer.target));
		}
		adoption
	}

	/// Paint `entity`'s subtree as fresh nodes, each bound to its entity,
	/// returning the top-level ones for the caller to place. A subtree
	/// already bound is reused, node and all, rather than painted again.
	pub fn render(
		&mut self,
		world: &mut World,
		entity: Entity,
	) -> Vec<web_sys::Node> {
		self.walk(world, entity, Frame::fresh()).0
	}

	/// One walk from `entity` under `frame`: the top-level nodes it produced
	/// and what it adopted, its bindings and the edits it found landed once
	/// the walk releases the world.
	fn walk(
		&mut self,
		world: &mut World,
		entity: Entity,
		frame: Frame,
	) -> (Vec<web_sys::Node>, Adoption) {
		let bound = world.query::<&DomNode>();
		let mut walker = SystemState::<NodeWalker>::new(world);
		let mut painter = DomPainter::new(&self.target, world, &bound, frame);
		walker
			.get(world)
			.expect("infallible node query")
			.walk(&mut painter, entity);
		let Painted {
			roots,
			bindings,
			edits,
			adoption,
		} = painter.finish();
		for (entity, binding) in bindings {
			world.entity_mut(entity).insert(binding);
		}
		// a control the user edited before the world existed: the world takes
		// its value, as it takes every later `input`
		for (entity, live) in edits {
			live.write(world, entity);
		}
		(roots, adoption)
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

/// Where one open element's children land: the element they are placed in
/// and, when it adopts, the served child the next one is expected to match.
struct Frame {
	/// The element children are placed in; `None` at the top of a fresh
	/// render, where they are collected as roots.
	parent: Option<web_sys::Element>,
	/// The served node the next child is matched against, advanced as each
	/// is adopted or replaced; `None` past the last, where a child is
	/// appended. Always `None` on a frame painting fresh.
	cursor: Option<web_sys::Node>,
	/// Whether the frame adopts the element's served children, or paints
	/// fresh into an element that has none.
	adopting: bool,
}

impl Frame {
	/// The top of a fresh render: children are roots.
	fn fresh() -> Self {
		Self {
			parent: None,
			cursor: None,
			adopting: false,
		}
	}

	/// Painting into a freshly created `element`.
	fn painting(element: web_sys::Element) -> Self {
		Self {
			parent: Some(element),
			cursor: None,
			adopting: false,
		}
	}

	/// Adopting `element`'s served children, from the first.
	fn adopting(element: &web_sys::Element) -> Self {
		Self {
			parent: Some(element.clone()),
			cursor: element.first_child(),
			adopting: true,
		}
	}
}

/// What one walk produced, handed back once it releases the world.
struct Painted {
	/// The top-level nodes, in order.
	roots: Vec<web_sys::Node>,
	/// The bindings the walk made.
	bindings: Vec<(Entity, DomNode)>,
	/// Controls the served page holds an edit in, with the value to write.
	edits: Vec<(Entity, LiveValue)>,
	adoption: Adoption,
}

/// The visitor of one [`DomRenderer`] walk: what it painted, adopted and
/// bound.
struct DomPainter<'w> {
	target: &'w web_sys::Element,
	document: web_sys::Document,
	world: &'w World,
	/// The bindings made before this walk, so a bound subtree is reused.
	bound: &'w QueryState<&'static DomNode>,
	/// The frames being painted into, innermost last; the root frame is the
	/// walk's own and is closed by [`Self::finish`].
	frames: Vec<Frame>,
	/// The top-level nodes the walk produced, in order.
	roots: Vec<web_sys::Node>,
	/// The bindings the walk made, applied once it holds the world again.
	bindings: Vec<(Entity, DomNode)>,
	/// Adopted controls holding an edit made before the world existed.
	edits: Vec<(Entity, LiveValue)>,
	adoption: Adoption,
	/// The value of the `<select>` being painted fresh, for the `<option>`
	/// it names; an adopted select's served options already carry theirs.
	select_value: Option<String>,
}

impl<'w> DomPainter<'w> {
	fn new(
		target: &'w web_sys::Element,
		world: &'w World,
		bound: &'w QueryState<&'static DomNode>,
		frame: Frame,
	) -> Self {
		Self {
			target,
			document: target
				.owner_document()
				.unwrap_or_else(document_ext::document),
			world,
			bound,
			frames: vec![frame],
			roots: default(),
			bindings: default(),
			edits: default(),
			adoption: default(),
			select_value: None,
		}
	}

	/// Close the root frame and hand back what the walk produced.
	fn finish(mut self) -> Painted {
		if let Some(frame) = self.frames.pop() {
			self.close(frame);
		}
		Painted {
			roots: self.roots,
			bindings: self.bindings,
			edits: self.edits,
			adoption: self.adoption,
		}
	}

	/// Place `node` where the walk stands: before the served node the frame
	/// expects next, at the end of the element being painted into, or as a
	/// root.
	fn place(&mut self, node: &web_sys::Node) {
		match self.frames.last() {
			Some(Frame {
				parent: Some(parent),
				cursor,
				..
			}) => {
				parent.insert_before(node, cursor.as_ref()).ok();
			}
			_ => self.roots.push(node.clone()),
		}
	}

	/// The served node at the cursor when `matches` accepts it, taken and
	/// counted as adopted; nothing otherwise, the cursor untouched.
	fn take_if(
		&mut self,
		matches: impl FnOnce(&web_sys::Node) -> bool,
	) -> Option<web_sys::Node> {
		let frame = self.frames.last_mut().filter(|frame| frame.adopting)?;
		let node = frame.cursor.clone().filter(|node| matches(node))?;
		frame.cursor = node.next_sibling();
		self.adoption.adopted += 1;
		Some(node)
	}

	/// The served node at the cursor when `matches` accepts it, else a
	/// mismatch: the served node in that place (if any) is removed and
	/// counted as replaced, so the caller's fresh paint lands where it stood.
	fn adopt(
		&mut self,
		matches: impl FnOnce(&web_sys::Node) -> bool,
	) -> Option<web_sys::Node> {
		if !self.frames.last().is_some_and(|frame| frame.adopting) {
			return None;
		}
		if let Some(node) = self.take_if(matches) {
			return Some(node);
		}
		let frame = self.frames.last_mut().expect("an adopting frame");
		let served = frame.cursor.take();
		frame.cursor = served.as_ref().and_then(web_sys::Node::next_sibling);
		if let Some(node) = &served
			&& let Some(parent) = node.parent_node()
		{
			parent.remove_child(node).ok();
		}
		debug!(
			"adoption replaced a served {:?}",
			served.map(|node| node.node_name())
		);
		self.adoption.replaced += 1;
		None
	}

	/// Close `frame`: whatever served children it never reached are the
	/// page's surplus, removed and counted.
	fn close(&mut self, frame: Frame) {
		let Frame {
			parent: Some(parent),
			mut cursor,
			adopting: true,
		} = frame
		else {
			return;
		};
		while let Some(node) = cursor {
			cursor = node.next_sibling();
			debug!("adoption removed a surplus {:?}", node.node_name());
			parent.remove_child(&node).ok();
			self.adoption.replaced += 1;
		}
	}

	/// Bind `entity` to the node it painted as, stamped so the node answers
	/// for it.
	fn bind(&mut self, entity: Entity, binding: DomNode) {
		DomNode::stamp(binding.get(), entity);
		self.bindings.push((entity, binding));
	}

	/// The namespace `tag` is created in: its own for an `<svg>` or `<math>`
	/// root, else its parent's unless that is html, which needs no naming.
	fn namespace_for(&self, tag: &str) -> Option<String> {
		let parent = self
			.frames
			.iter()
			.rev()
			.find_map(|frame| frame.parent.as_ref())
			.unwrap_or(self.target);
		match tag {
			"svg" => Some(SVG_NAMESPACE.into()),
			"math" => Some(MATHML_NAMESPACE.into()),
			_ => parent
				.namespace_uri()
				.filter(|namespace| namespace != XHTML_NAMESPACE),
		}
	}

	/// The element `tag` creates in `namespace`; an unusable tag paints as a
	/// `<span>` and says so.
	fn create_element(
		&self,
		tag: &str,
		namespace: Option<&str>,
	) -> web_sys::Element {
		let created = match namespace {
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

	/// Whether a served element is the one `tag` would create: the same
	/// name (html is case-insensitive, other namespaces are not) in the
	/// same namespace.
	fn element_matches(
		element: &web_sys::Element,
		tag: &str,
		namespace: Option<&str>,
	) -> bool {
		let served = element.namespace_uri();
		let html = served.as_deref().is_none_or(|ns| ns == XHTML_NAMESPACE);
		let same_namespace = match namespace {
			Some(namespace) => served.as_deref() == Some(namespace),
			None => html,
		};
		same_namespace
			&& match html {
				true => element.tag_name().eq_ignore_ascii_case(tag),
				false => element.tag_name() == tag,
			}
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
		self.bind_attributes(element, view);
	}

	/// Bind each attribute entity to the element it sets.
	fn bind_attributes(
		&mut self,
		element: &web_sys::Element,
		view: &ElementView,
	) {
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

	/// An adopted element's attributes: only what the world states is
	/// touched, each set when the served element disagrees, and the merged
	/// `class` compared whole. Its attribute entities bind to it either way.
	fn adopt_attributes(
		&mut self,
		element: &web_sys::Element,
		view: &ElementView,
	) {
		for attr in &view.attributes {
			if attr.key() == "class" {
				continue;
			}
			let text = DomRenderer::text_of(attr.value);
			let served = element.get_attribute(attr.key());
			if served.as_deref() != Some(&text) {
				self.patched(view.tag(), attr.key(), served.as_deref(), &text);
				element.set_attribute(attr.key(), &text).ok();
			}
		}
		self.bind_attributes(element, view);
		let class = view.class_attribute();
		let served = element.get_attribute("class");
		if served != class {
			self.patched(
				view.tag(),
				"class",
				served.as_deref(),
				class.as_deref().unwrap_or_default(),
			);
			match class {
				Some(class) => {
					element.set_attribute("class", &class).ok();
				}
				None => {
					element.remove_attribute("class").ok();
				}
			}
		}
	}

	/// Count a patch, naming it for the person tuning a page's conformance.
	fn patched(
		&mut self,
		tag: &str,
		what: &str,
		served: Option<&str>,
		world: &str,
	) {
		debug!(
			"adoption patched <{tag}> {what}: served {served:?}, world {world:?}"
		);
		self.adoption.patched += 1;
	}

	/// An adopted control's value is read, never written: the served markup
	/// already carries the world's, and what the user changed before the
	/// world existed is the world's to take.
	fn adopt_value(&mut self, entity: Entity, element: &web_sys::Element) {
		if let Some(live) = LiveValue::edited(element) {
			self.edits.push((entity, live));
		}
	}

	/// The served text node at the cursor, adopted: split where the world's
	/// text ends when adjacent text entities parsed as one node, or its data
	/// set (and counted) when it differs. An empty text, which no served
	/// page carries a node for, is created without a count.
	fn adopt_text(&mut self, text: &str) -> Option<web_sys::Node> {
		let frame = self.frames.last().filter(|frame| frame.adopting)?;
		let served = frame
			.cursor
			.as_ref()
			.and_then(|node| node.dyn_ref::<web_sys::Text>())
			.cloned();
		let Some(served) = served else {
			return match text.is_empty() {
				true => None,
				false => self.adopt(|_| false),
			};
		};
		let data = served.data();
		if data != text {
			if data.starts_with(text) {
				served.split_text(text.encode_utf16().count() as u32).ok();
			} else {
				self.patched("text", "data", Some(&data), text);
				served.set_data(text);
			}
		}
		self.take_if(|_| true)
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
			if self.adopt(|served| *served == node).is_none() {
				self.place(&node);
			}
			return true;
		}
		let Some(tag) = element
			.map(|element| element.tag())
			.filter(|tag| DomRenderer::is_skipped(tag))
		else {
			return false;
		};
		// a skipped tag the served page carries is bound where it stands, so
		// a later reconcile keeps it; one it lacks is never painted
		if let Some(node) = self.take_if(|node| {
			node.dyn_ref::<web_sys::Element>().is_some_and(|element| {
				Self::element_matches(element, tag, None)
			})
		}) {
			self.bind(cx.entity, DomNode::node(node));
		}
		true
	}

	fn visit_comment(&mut self, cx: &VisitContext, comment: &Comment) {
		// functional `bx` anchors paint; authoring comments never reach a page
		if !comment.starts_with("bx") {
			return;
		}
		let node = match self
			.adopt(|node| node.dyn_ref::<web_sys::Comment>().is_some())
		{
			Some(node) => {
				let served = node.unchecked_ref::<web_sys::Comment>();
				let data = served.data();
				if data != **comment {
					self.patched("comment", "data", Some(&data), comment);
					served.set_data(comment);
				}
				node
			}
			None => {
				let node = self.document.create_comment(comment);
				self.place(&node);
				node.into()
			}
		};
		self.bind(cx.entity, DomNode::node(node));
	}

	fn visit_element(&mut self, cx: &VisitContext, view: ElementView) {
		match view.tag() {
			// the document's own: transparent, and the target stands in
			"html" => return,
			"body" => {
				let target = self.target.clone();
				self.adopt_attributes(&target, &view);
				self.bind(cx.entity, DomNode::document(target));
				return;
			}
			_ => {}
		}
		let namespace = self.namespace_for(view.tag());
		let adopted = self.adopt(|node| {
			node.dyn_ref::<web_sys::Element>().is_some_and(|element| {
				Self::element_matches(element, view.tag(), namespace.as_deref())
			})
		});
		let element = match adopted {
			Some(node) => {
				let element = node.unchecked_into::<web_sys::Element>();
				self.adopt_attributes(&element, &view);
				if is_value_element(view.tag()) {
					self.adopt_value(cx.entity, &element);
				}
				// a textarea's served child is its value, never surplus
				self.frames.push(match view.tag() {
					"textarea" => Frame::painting(element.clone()),
					_ => Frame::adopting(&element),
				});
				element
			}
			None => {
				let element =
					self.create_element(view.tag(), namespace.as_deref());
				self.apply_attributes(&element, &view);
				self.apply_value(&element, &view);
				self.apply_class(&element, &view);
				self.place(&element);
				self.frames.push(Frame::painting(element.clone()));
				element
			}
		};
		self.bind(cx.entity, DomNode::node(element));
	}

	fn leave_element(&mut self, _cx: &VisitContext, element: &Element) {
		if matches!(element.tag(), "html" | "body") {
			return;
		}
		let Some(frame) = self.frames.pop() else {
			return;
		};
		let painted = frame.parent.clone().filter(|_| !frame.adopting);
		self.close(frame);
		// a fresh select's value lands once its options exist
		if element.tag() == "select"
			&& let Some(value) = self.select_value.take()
			&& let Some(select) = painted.and_then(|element| {
				element.dyn_into::<web_sys::HtmlSelectElement>().ok()
			}) {
			select.set_value(&value);
		}
	}

	fn visit_value(&mut self, cx: &VisitContext, value: &Value) {
		let text = value.to_string();
		let node = match self.adopt_text(&text) {
			Some(node) => node,
			None => {
				let node = self.document.create_text_node(&text);
				self.place(&node);
				node.into()
			}
		};
		self.bind(cx.entity, DomNode::node(node));
	}
}

#[cfg(test)]
mod test {
	use super::super::test_ext::*;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use wasm_bindgen::JsCast;

	/// The control-heavy tree the probes run over.
	fn controls_page(app: &mut App) -> Entity {
		let host = build(app, rsx! {
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
		set_document(
			app,
			host,
			value!({
				"name": "pete",
				"bed": "beets",
				"done": true,
			}),
		);
		// the bindings sync, then the checkbox mirrors its `checked`
		app.update();
		app.update();
		host
	}

	/// The parity probe: what the DOM sink paints serializes exactly as the
	/// string sink writes, through attributes in order, merged classes,
	/// escaped text, a control's value, a checkbox's `checked` and a select's
	/// marked option.
	#[beet_core::test(browser)]
	fn paints_what_the_string_sink_writes() {
		let mut app = dom_app();
		let host = controls_page(&mut app);
		let target = paint(&mut app, host);
		target
			.inner_html()
			.xpect_eq(ssr_html(app.world_mut(), host));
	}

	/// The conformance probe: the served page, parsed by the browser, adopts
	/// with nothing replaced or patched, every entity bound to the served
	/// node that stood there, and an empty target is a plain first paint.
	#[beet_core::test(browser)]
	fn adopts_the_served_page_untouched() {
		let mut app = dom_app();
		let host = controls_page(&mut app);
		let target = serve(&mut app, host);
		let served = target.query_selector("h1").unwrap().unwrap();
		let adoption = adopt(&mut app, host, &target);
		adoption.is_clean().xpect_true();
		adoption.adopted.xpect_greater_than(10);
		let heading = element(app.world_mut(), "h1");
		node_of(app.world(), heading)
			.xpect_eq(served.dyn_into::<web_sys::Node>().unwrap());
		target
			.inner_html()
			.xpect_eq(ssr_html(app.world_mut(), host));
		// a fresh target is painted rather than adopted, and says so
		let mut app = dom_app();
		let host = controls_page(&mut app);
		let fresh = container();
		let adoption = adopt(&mut app, host, &fresh);
		adoption.adopted.xpect_eq(0);
		adoption.replaced.xpect_greater_than(0);
		fresh.inner_html().xpect_eq(ssr_html(app.world_mut(), host));
	}

	/// A served node the world disagrees with is replaced in its place and
	/// counted, a differing text or attribute is set and counted, surplus
	/// served nodes go, and the result is the world's page either way.
	#[beet_core::test(browser)]
	fn a_mismatch_is_replaced_in_place() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<ul><li id="a">"a"</li><li id="b">"b"</li></ul>
		});
		let target = container();
		target.set_inner_html(
			"<ul><li id=\"x\">a</li><p>stale</p><li id=\"c\">b</li></ul><footer></footer>",
		);
		let kept = target.query_selector("li").unwrap().unwrap();
		let adoption = adopt(&mut app, host, &target);
		// the first `<li>` adopted with its id patched, the `<p>` replaced by
		// a fresh `<li>`, the surplus `<li>` and `<footer>` removed
		adoption.replaced.xpect_eq(3);
		adoption.patched.xpect_eq(1);
		target
			.inner_html()
			.xpect_eq(ssr_html(app.world_mut(), host));
		let first = element(app.world_mut(), "li");
		node_of(app.world(), first)
			.xpect_eq(kept.dyn_into::<web_sys::Node>().unwrap());
	}

	/// Adjacent text entities serve as one text node, which adoption splits
	/// where the first ends; a text the served page disagrees with is set.
	#[beet_core::test(browser)]
	fn adjacent_texts_split_the_served_node() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<p>{"Every "}{"entity"}</p>
		});
		let target = serve(&mut app, host);
		adopt(&mut app, host, &target).is_clean().xpect_true();
		let paragraph = element(app.world_mut(), "p");
		let texts: Vec<Entity> = app
			.world()
			.get::<Children>(paragraph)
			.unwrap()
			.iter()
			.collect();
		node_of(app.world(), texts[0])
			.text_content()
			.xpect_eq(Some("Every ".to_string()));
		node_of(app.world(), texts[1])
			.text_content()
			.xpect_eq(Some("entity".to_string()));
		// a stale text is patched in place
		let target = container();
		target.set_inner_html("<p>Every thing</p>");
		let served = target.query_selector("p").unwrap().unwrap().first_child();
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<p>{"Every "}{"entity"}</p>
		});
		let adoption = adopt(&mut app, host, &target);
		adoption.patched.xpect_eq(1);
		adoption.replaced.xpect_eq(0);
		let paragraph = element(app.world_mut(), "p");
		let first = app.world().get::<Children>(paragraph).unwrap()[0];
		node_of(app.world(), first).xpect_eq(served.unwrap());
		target
			.text_content()
			.xpect_eq(Some("Every entity".to_string()));
	}

	/// A control the user edited before the world existed hands its value
	/// to the world; one left as served writes nothing.
	#[beet_core::test(browser)]
	fn an_edited_control_lands_in_the_world() {
		let mut app = dom_app();
		let host = controls_page(&mut app);
		let target = serve(&mut app, host);
		let input = target
			.query_selector("input[type=text]")
			.unwrap()
			.unwrap()
			.dyn_into::<web_sys::HtmlInputElement>()
			.unwrap();
		let checkbox = target
			.query_selector("input[type=checkbox]")
			.unwrap()
			.unwrap()
			.dyn_into::<web_sys::HtmlInputElement>()
			.unwrap();
		let select = target
			.query_selector("select")
			.unwrap()
			.unwrap()
			.dyn_into::<web_sys::HtmlSelectElement>()
			.unwrap();
		// the visitor typed, unticked and re-picked before the wasm arrived
		input.set_value("peter");
		checkbox.set_checked(false);
		select.set_value("carrots");
		adopt(&mut app, host, &target).is_clean().xpect_true();
		app.update();
		field(app.world(), host, "name").xpect_eq(Value::str("peter"));
		field(app.world(), host, "done").xpect_eq(Value::Bool(false));
		field(app.world(), host, "bed").xpect_eq(Value::str("carrots"));
		// the controls keep what they hold: nothing was reassigned
		input.value().xpect_eq("peter");
		// an untouched page writes nothing back
		let mut app = dom_app();
		let host = controls_page(&mut app);
		let target = serve(&mut app, host);
		adopt(&mut app, host, &target);
		app.update();
		field(app.world(), host, "name").xpect_eq(Value::str("pete"));
	}

	/// The conformance probe over prose: a markdown page with a table, a
	/// fenced block and inline code, the shapes a browser's parser is most
	/// tempted to reshape, adopts untouched.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test(browser)]
	fn a_markdown_page_adopts_untouched() {
		let mut app = dom_app();
		let host = app.world_mut().spawn_empty().id();
		let markdown = MediaBytes::new_markdown(
			"# Routing\n\n`<RoutesDir/>` scans:\n\n| file | route |\n| --- | --- |\n| `a.md` | `/a` |\n| `b/c.md` | `/b/c` |\n\nFrontmatter:\n\n```toml\n+++\ntitle = \"Routing\"\n+++\n```\n\n- one\n- two\n",
		);
		MarkdownParser::new()
			.parse(ParseContext::new(
				&mut app.world_mut().entity_mut(host),
				&markdown,
			))
			.unwrap();
		let target = serve(&mut app, host);
		adopt(&mut app, host, &target).is_clean().xpect_true();
		target
			.inner_html()
			.xpect_eq(ssr_html(app.world_mut(), host));
	}

	/// The scene editor page forked and opened, the tree the browser suite
	/// edits.
	#[cfg(feature = "template_serde")]
	fn scene_editor_page(app: &mut App) -> Entity {
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
		host
	}

	/// The probe over the scene editor page itself, editor open: the page
	/// the browser suite edits paints byte-equal to its SSR.
	#[cfg(feature = "template_serde")]
	#[beet_core::test(browser)]
	fn the_scene_editor_page_paints_byte_equal() {
		let mut app = dom_app();
		let host = scene_editor_page(&mut app);
		let target = paint(&mut app, host);
		target
			.inner_html()
			.xpect_eq(ssr_html(app.world_mut(), host));
	}

	/// The conformance probe over the scene editor page: served and parsed
	/// by the browser, it adopts untouched.
	#[cfg(feature = "template_serde")]
	#[beet_core::test(browser)]
	fn the_scene_editor_page_adopts_untouched() {
		let mut app = dom_app();
		let host = scene_editor_page(&mut app);
		let target = serve(&mut app, host);
		adopt(&mut app, host, &target).is_clean().xpect_true();
	}
}
