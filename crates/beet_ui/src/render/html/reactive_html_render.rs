//! The thin-client reactivity wire format: the annotations [`HtmlRenderer`] emits
//! in reactive mode ([`HtmlRenderer::reactive`]) so a browser runtime can hydrate
//! and drive a page with no WASM. This module is the single source of truth for
//! that format; the JavaScript runtime ([`REACTIVITY_JS`]) mirrors it.
//!
//! The divide is deliberate: the JSON blob carries **state** (the typed values),
//! the in-place markers carry **topology** (which node binds to which path), and
//! no value lives in both. The SSR text is first-paint only; on load the runtime
//! trusts the blob, so there is no re-render flash.
//!
//! ## Vocabulary
//!
//! - `data-bx-doc="dN"`: marks the outermost element of document N's subtree. The
//!   runtime walks up from a bound node to find its document.
//! - `<!--bx-ref="PATH"-->TEXT<!--bx-end-->`: a bound text run; `PATH` is the
//!   dotted absolute path within the enclosing `data-bx-doc`.
//! - `data-bx-attr-NAME="PATH"`: element attribute `NAME` is bound to `PATH`.
//! - `bx:EVENT="VERB{ ARG: VAL, .. }"`: an event verb trigger; `VAL` is
//!   `@doc:PATH`/`@prop:PATH` for a binding, else a JSON literal.
//! - `<script type="application/json" data-bx-blob>{"d0": {..}}</script>`: the
//!   initial document state, keyed by document id, the hydration source.
//! - `<script type="application/json" data-bx-verbs>{"name": "js src"}</script>`:
//!   the JS verbs to install (omitted when none).
//!
//! The blob, the verbs, and the runtime `<script>` are injected into `<head>`
//! during the single render walk (a post-walk append is the fallback for a
//! fragment with no `<head>`). Only `@doc`/`@prop` document state is in scope;
//! `@res`/`@comp` (reflect) are server-rendered only, a later WASM concern.

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemState;

/// The thin-client reactivity runtime source (`reactivity.js`), the single asset
/// the static route serves and the inline render embeds.
pub const REACTIVITY_JS: &str = include_str!("./reactivity.js");

/// The default URL the runtime `<script defer>` loads from, the single source
/// shared by [`ReactiveHtmlRender`] and the static route serving
/// [`REACTIVITY_JS`].
pub const REACTIVITY_SRC: &str = "/js/reactivity.js";

/// When [`ReactiveHtmlRender`] injects the runtime script.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InsertReactive {
	/// Emit the runtime + blob only when the page has reactive content.
	#[default]
	Auto,
	/// Always inject the runtime script, even on a plain page, so reactivity
	/// injected at runtime later still has its runtime.
	Always,
}

/// Reactivity annotations collected from the world before a reactive render, plus
/// the config and walk-time state for emitting them, the engine of
/// [`HtmlRenderer::reactive`]. See the [module docs](self) for the wire format.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ReactiveHtmlRender {
	/// When to inject the runtime script (see [`InsertReactive`]).
	insert_reactive: InsertReactive,
	/// `false` (default): emit `<script defer src="/js/reactivity.js">`. `true`:
	/// inline [`REACTIVITY_JS`] as-is instead.
	inline_js_runtime: bool,
	/// Document id -> its value, the hydration blob (referenced documents only).
	blob: Vec<(usize, Value)>,
	/// Every (`ChildOf`-reachable) entity -> the id of its governing document.
	governing: HashMap<Entity, usize>,
	/// Document ids actually referenced by a binding, so unrelated server-side
	/// documents are never shipped or marked.
	referenced: HashSet<usize>,
	/// A bound text-node entity -> its dotted absolute path within its document.
	text_paths: HashMap<Entity, String>,
	/// A bound attribute entity -> its dotted absolute path.
	attr_paths: HashMap<Entity, String>,
	/// An element entity -> its `bx:<event>` triggers as `(event, verb-call)`.
	events: HashMap<Entity, Vec<(SmolStr, String)>>,
	/// The JS verbs to install client-side as `(name, source)`.
	verbs: Vec<(SmolStr, String)>,
	/// Walk state: the governing document of each enclosing visited element.
	element_doc_stack: Vec<Option<usize>>,
	/// Walk state: set once the `<head>` injection has been emitted, so the
	/// post-walk fallback only fires for a fragment with no `<head>`.
	head_injected: bool,
}

impl ReactiveHtmlRender {
	/// Build a reactive renderer state with the given injection mode and runtime
	/// embedding.
	pub(crate) fn new(
		insert_reactive: InsertReactive,
		inline_js_runtime: bool,
	) -> Self {
		Self {
			insert_reactive,
			inline_js_runtime,
			..Default::default()
		}
	}
	/// Collect every annotation for the render rooted at `root`, the one pass that
	/// reads the world (the visit-time methods are pure lookups).
	pub(crate) fn collect(&mut self, world: &mut World, root: Entity) {
		let mut doc_ids = HashMap::<Entity, usize>::default();
		let mut blob = Vec::<Value>::new();
		self.walk_docs(world, root, None, &mut doc_ids, &mut blob);
		self.collect_paths(world);
		self.collect_events(world);
		self.collect_referenced(world);
		// keep only the documents a binding actually reads.
		self.blob = blob
			.into_iter()
			.enumerate()
			.filter(|(id, _)| self.referenced.contains(id))
			.collect();
		self.verbs = world
			.get_resource::<VerbRegistry>()
			.map(VerbRegistry::js_verbs)
			.unwrap_or_default();
	}

	/// Walk the render tree (transparent through [`Portal`]), assigning each
	/// non-props [`Document`] a tree-order id and recording each entity's
	/// governing document.
	fn walk_docs(
		&mut self,
		world: &mut World,
		entity: Entity,
		current: Option<usize>,
		doc_ids: &mut HashMap<Entity, usize>,
		blob: &mut Vec<Value>,
	) {
		// a Portal holder is transparent: descend into the referenced entity,
		// mirroring the node walker. Transcluded content carries its own document
		// context (the document may live on the content tree's root, *above* this
		// transclusion entry), so resolve it from the target's own ancestors rather
		// than inheriting the holder's; the entry element then tops the document.
		if let Some(target) = world.get::<Portal>(entity).map(Portal::target) {
			let doc = self
				.governing_doc_id(world, target, doc_ids, blob)
				.or(current);
			self.walk_docs(world, target, doc, doc_ids, blob);
			return;
		}
		let mut doc = current;
		if let Some(id) = self.assign_doc_id(world, entity, doc_ids, blob) {
			doc = Some(id);
		}
		if let Some(id) = doc {
			self.governing.insert(entity, id);
		}
		let children = world
			.get::<Children>(entity)
			.map(|children| children.iter().collect::<Vec<_>>());
		if let Some(children) = children {
			for child in children {
				self.walk_docs(world, child, doc, doc_ids, blob);
			}
		}
	}

	/// The blob id of `entity`'s own [`Document`] (skipping a [`PropsDocument`]),
	/// assigning a fresh tree-order id and capturing its value on first sight.
	fn assign_doc_id(
		&self,
		world: &World,
		entity: Entity,
		doc_ids: &mut HashMap<Entity, usize>,
		blob: &mut Vec<Value>,
	) -> Option<usize> {
		if world.get::<Document>(entity).is_none()
			|| world.get::<PropsDocument>(entity).is_some()
		{
			return None;
		}
		let id = *doc_ids.entry(entity).or_insert_with(|| {
			let id = blob.len();
			blob.push(world.get::<Document>(entity).unwrap().0.clone());
			id
		});
		Some(id)
	}

	/// The document governing a transclusion target: the nearest non-props
	/// [`Document`] at or above it in the `ChildOf` tree (the content tree's root
	/// often holds the document above the transclusion entry).
	fn governing_doc_id(
		&self,
		world: &World,
		target: Entity,
		doc_ids: &mut HashMap<Entity, usize>,
		blob: &mut Vec<Value>,
	) -> Option<usize> {
		let mut current = Some(target);
		while let Some(entity) = current {
			if let Some(id) = self.assign_doc_id(world, entity, doc_ids, blob) {
				return Some(id);
			}
			current = world
				.get::<ChildOf>(entity)
				.map(|child_of| child_of.parent());
		}
		None
	}

	/// Record the dotted path of every bound text node and attribute entity, keyed
	/// by entity for the visit-time lookup.
	fn collect_paths(&mut self, world: &mut World) {
		let mut state = SystemState::<(
			Query<(
				Entity,
				&ResolvedFieldPath,
				Option<&Element>,
				Option<&AttributeOf>,
				Option<&FieldOf>,
			)>,
			Query<(), With<PropsDocument>>,
		)>::new(world);
		let (query, props) =
			state.get(world).expect("infallible binding query");
		for (entity, resolved, element, attribute_of, field_of) in query.iter()
		{
			// a binding into a props store is server-only: a props store holds a
			// template's materialized props and is never shipped, so the client
			// cannot read it (the SSR value is final). Skip it, like `@res`/`@comp`.
			if field_of
				.is_some_and(|field_of| props.contains(field_of.document))
			{
				continue;
			}
			let dotted = dotted_path(&resolved.field_path);
			if let Some(attribute_of) = attribute_of {
				self.attr_paths.insert(entity, dotted);
				// an attribute entity is outside the ChildOf tree: inherit its
				// owning element's governing document.
				if let Some(&doc) = self.governing.get(&attribute_of.get()) {
					self.governing.insert(entity, doc);
				}
			} else if element.is_none() {
				// a pure text node (no element) is a `<!--bx-ref-->` run.
				self.text_paths.insert(entity, dotted);
			}
		}
	}

	/// Record each entity's `bx:<event>` triggers, resolving every `@doc`/`@prop`
	/// argument to its absolute path so the client needs no scope walk.
	fn collect_events(&mut self, world: &mut World) {
		let mut state = SystemState::<(
			Query<(Entity, &EventBindings)>,
			DocumentResolver,
			ScopeQuery,
		)>::new(world);
		let (query, resolver, scopes) =
			state.get(world).expect("infallible event query");
		for (entity, bindings) in query.iter() {
			for binding in &bindings.0 {
				let call =
					render_verb_call(binding, entity, &resolver, &scopes);
				self.events
					.entry(entity)
					.or_default()
					.push((binding.event.clone(), call));
			}
		}
	}

	/// Compute the set of documents a binding actually reads (text, attribute, or
	/// event), so the blob and `data-bx-doc` markers ship only needed state.
	fn collect_referenced(&mut self, _world: &mut World) {
		let bound = self
			.text_paths
			.keys()
			.chain(self.attr_paths.keys())
			.chain(self.events.keys());
		for entity in bound {
			if let Some(&doc) = self.governing.get(entity) {
				self.referenced.insert(doc);
			}
		}
	}

	/// On entering an element: push its governing document and return the id to
	/// emit as `data-bx-doc` when this element tops a referenced document region.
	pub(crate) fn enter_element(&mut self, entity: Entity) -> Option<usize> {
		let governing = self.governing.get(&entity).copied();
		let enclosing = self.element_doc_stack.last().copied().flatten();
		let emit = governing.filter(|id| {
			self.referenced.contains(id) && Some(*id) != enclosing
		});
		self.element_doc_stack.push(governing);
		emit
	}

	/// On leaving an element: pop its governing document.
	pub(crate) fn leave_element(&mut self) { self.element_doc_stack.pop(); }

	/// The `(event, verb-call)` triggers to emit on this element.
	pub(crate) fn events(&self, entity: Entity) -> &[(SmolStr, String)] {
		self.events.get(&entity).map_or(&[], Vec::as_slice)
	}

	/// The bound path of an attribute entity, if any.
	pub(crate) fn attr_path(&self, entity: Entity) -> Option<&str> {
		self.attr_paths.get(&entity).map(String::as_str)
	}

	/// The bound path of a text-node entity, if any.
	pub(crate) fn text_path(&self, entity: Entity) -> Option<&str> {
		self.text_paths.get(&entity).map(String::as_str)
	}

	/// The `<script>` carrying the hydration blob, `None` when no document is
	/// referenced.
	pub(crate) fn blob_script(&self) -> Option<String> {
		if self.blob.is_empty() {
			return None;
		}
		let map = self
			.blob
			.iter()
			.map(|(id, value)| {
				(format!("d{id}"), serde_json::Value::from(value.clone()))
			})
			.collect::<serde_json::Map<_, _>>();
		let json = serde_json::to_string(&serde_json::Value::Object(map))
			.unwrap_or_else(|_| "{}".into());
		Some(format!(
			"<script type=\"application/json\" data-bx-blob>{}</script>",
			escape_script_json(&json)
		))
	}

	/// The `<script>` carrying the JS verbs to install (the defaults plus any app
	/// verbs), `None` when none ship a twin. The runtime has no built-in verbs, so
	/// these are the page's whole vocabulary.
	pub(crate) fn verbs_script(&self) -> Option<String> {
		if self.verbs.is_empty() {
			return None;
		}
		let map = self
			.verbs
			.iter()
			.map(|(name, source)| {
				(name.to_string(), serde_json::Value::String(source.clone()))
			})
			.collect::<serde_json::Map<_, _>>();
		let json = serde_json::to_string(&serde_json::Value::Object(map))
			.unwrap_or_else(|_| "{}".into());
		Some(format!(
			"<script type=\"application/json\" data-bx-verbs>{}</script>",
			escape_script_json(&json)
		))
	}

	/// The runtime `<script>`: an external `<script defer src>` by default, or
	/// the inlined [`REACTIVITY_JS`] when
	/// [`inline_js_runtime`](Self::inline_js_runtime) is set.
	fn runtime_script(&self) -> String {
		if self.inline_js_runtime {
			format!("<script>{REACTIVITY_JS}</script>")
		} else {
			format!("<script defer src=\"{REACTIVITY_SRC}\"></script>")
		}
	}

	/// Whether the *page* carries a reactive annotation (a bound text run or
	/// attribute, or an event trigger), the `Auto`-mode gate. The verb vocabulary
	/// is global (always registered), so it is never the signal here; it ships
	/// only alongside a page that is already reactive.
	fn has_reactive_content(&self) -> bool {
		!self.blob.is_empty()
			|| !self.text_paths.is_empty()
			|| !self.attr_paths.is_empty()
			|| !self.events.is_empty()
	}

	/// The `<head>` injection (blob + verbs + runtime script) to emit once, or
	/// `None` in `Auto` mode for a page with no reactive content. Marks the
	/// injection consumed, so the caller emits it exactly once: at `</head>`, or
	/// the post-walk fallback for a fragment with no `<head>`.
	pub(crate) fn take_head_injection(&mut self) -> Option<String> {
		if self.head_injected {
			return None;
		}
		self.head_injected = true;
		let emit = match self.insert_reactive {
			InsertReactive::Always => true,
			InsertReactive::Auto => self.has_reactive_content(),
		};
		if !emit {
			return None;
		}
		let mut out = String::new();
		if let Some(blob) = self.blob_script() {
			out.push_str(&blob);
		}
		if let Some(verbs) = self.verbs_script() {
			out.push_str(&verbs);
		}
		out.push_str(&self.runtime_script());
		Some(out)
	}
}

/// Reconstruct a `verb{ arg: value, .. }` call for a `bx:<event>` attribute,
/// resolving binding arguments to absolute document paths.
fn render_verb_call(
	binding: &EventBinding,
	entity: Entity,
	resolver: &DocumentResolver,
	scopes: &ScopeQuery,
) -> String {
	if binding.args.is_empty() {
		return binding.verb.to_string();
	}
	let args = binding
		.args
		.iter()
		.map(|(name, arg)| {
			format!(
				"{name}: {}",
				render_verb_arg(arg, entity, resolver, scopes)
			)
		})
		.collect::<Vec<_>>()
		.join(", ");
	format!("{}{{ {args} }}", binding.verb)
}

/// Serialize a single verb argument: a binding to `@source:absolute.path`, a
/// literal to its JSON value.
fn render_verb_arg(
	arg: &VerbArg,
	entity: Entity,
	resolver: &DocumentResolver,
	scopes: &ScopeQuery,
) -> String {
	match arg {
		VerbArg::Binding(expr) => {
			let (prefix, document) = match expr.source {
				BindingSource::Doc => ("@doc:", DocumentPath::Ancestor),
				BindingSource::Prop => ("@prop:", DocumentPath::Props),
				// reflect sources are server-only; emit best-effort so the markup
				// stays readable, the client ignores them.
				BindingSource::Res => {
					return format!("@res:{}", expr.field_path);
				}
				BindingSource::Comp => {
					return format!("@comp:{}", expr.field_path);
				}
			};
			let doc_entity = resolver.entity(entity, &document);
			let resolved = scopes.resolved_path(
				entity,
				&expr.field_path,
				Some(doc_entity),
			);
			format!("{prefix}{}", dotted_path(&resolved))
		}
		VerbArg::Literal(literal) => literal_value(literal)
			.map(|value| {
				serde_json::to_string(&serde_json::Value::from(value))
					.unwrap_or_else(|_| "null".into())
			})
			.unwrap_or_else(|| "null".into()),
	}
}

/// A [`FieldPath`] as a dotted string with bare numeric array indices
/// (`items.0.name`), the form the client splits on `.`.
fn dotted_path(path: &FieldPath) -> String {
	path.iter()
		.map(|segment| match segment {
			FieldSegment::ArrayIndex(index) => index.to_string(),
			FieldSegment::ObjectKey(key) => key.to_string(),
		})
		.collect::<Vec<_>>()
		.join(".")
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A counter document: scoped state, a bound text run, and verb triggers, the
	/// `bsx_site` counter in miniature.
	const COUNTER: &str = r#"<article bx:scope="counter">
	<p>You have clicked {@doc:count=0} times.</p>
	<button bx:click=increment{ field: @doc:count, amount: 1 }>More</button>
	<button bx:click=decrement{ field: @doc:count }>Less</button>
</article>"#;

	/// The [`COUNTER`] inside a full document, so the head-injection point exists.
	const COUNTER_PAGE: &str = r#"<html><head><title>t</title></head><body><article bx:scope="counter"><p>You have clicked {@doc:count=0} times.</p></article></body></html>"#;

	/// A document with no bindings, ie nothing reactive to ship.
	const PLAIN_PAGE: &str = r#"<html><head><title>t</title></head><body><p>hello</p></body></html>"#;

	/// Build `markup` as a document and settle the sync so its documents exist.
	fn build(world: &mut World, markup: &str) -> Entity {
		let container = world
			.spawn_template(BsxTemplate::container(
				parse_document(markup, &BsxParseConfig::bsx()).unwrap(),
				BsxTemplateRegistry::default(),
			))
			.unwrap()
			.id();
		world.update_local();
		world.update_local();
		container
	}

	fn reactive_html(world: &mut World, root: Entity) -> String {
		HtmlRenderer::new()
			.reactive()
			.render(&mut RenderContext::new(root, world))
			.unwrap()
			.to_string()
	}

	/// Reactive render with explicit [`InsertReactive`] / inline config.
	fn reactive_html_with(
		world: &mut World,
		root: Entity,
		insert: InsertReactive,
		inline: bool,
	) -> String {
		HtmlRenderer::new()
			.reactive_with(insert, inline)
			.render(&mut RenderContext::new(root, world))
			.unwrap()
			.to_string()
	}

	fn static_html(world: &mut World, root: Entity) -> String {
		HtmlRenderer::new()
			.render(&mut RenderContext::new(root, world))
			.unwrap()
			.to_string()
	}

	#[beet_core::test]
	fn wraps_bound_text_run_keeping_static_text() {
		let mut world = ui_world();
		let root = build(&mut world, COUNTER);
		// the dynamic run is wrapped, the surrounding static text untouched.
		reactive_html(&mut world, root).xpect_contains(
			"You have clicked <!--bx-ref=\"counter.count\"-->0<!--bx-end--> times.",
		);
	}

	#[beet_core::test]
	fn marks_document_and_emits_blob() {
		let mut world = ui_world();
		let root = build(&mut world, COUNTER);
		reactive_html(&mut world, root)
			.xpect_contains("data-bx-doc=\"d0\"")
			.xpect_contains(
				"<script type=\"application/json\" data-bx-blob>{\"d0\":{\"counter\":{\"count\":0}}}</script>",
			);
	}

	#[beet_core::test]
	fn reemits_event_verbs_with_resolved_paths() {
		let mut world = ui_world();
		let root = build(&mut world, COUNTER);
		// the `@doc:count` arg resolves through the `counter` scope to an absolute
		// path, so the client needs no scope walk.
		reactive_html(&mut world, root)
			.xpect_contains(
				"bx:click=\"increment{ field: @doc:counter.count, amount: 1 }\"",
			)
			.xpect_contains(
				"bx:click=\"decrement{ field: @doc:counter.count }\"",
			);
	}

	#[beet_core::test]
	fn static_render_has_no_annotations() {
		let mut world = ui_world();
		let root = build(&mut world, COUNTER);
		static_html(&mut world, root)
			.xnot()
			.xpect_contains("bx-ref")
			.xnot()
			.xpect_contains("data-bx")
			.xnot()
			.xpect_contains("bx:click");
	}

	#[beet_core::test]
	fn emits_default_verbs() {
		let mut world = ui_world();
		let root = build(&mut world, COUNTER);
		// the runtime ships zero built-in verbs, so every default verb's JS twin is
		// emitted into `data-bx-verbs` for the client to install.
		reactive_html(&mut world, root)
			.xpect_contains("data-bx-verbs")
			.xpect_contains("entity.set_field(args.field, current + amount)");
	}

	#[beet_core::test]
	fn emits_app_registered_js_verb() {
		let mut world = ui_world();
		// an app verb supplies its JS twin in the same registration.
		world.resource_mut::<VerbRegistry>().insert(
			"shout",
			VerbSchema::new().binding("field"),
			Some("entity.set_field(args.field, 'HEY');"),
			|_, _| {},
		);
		let root = build(
			&mut world,
			r#"<article><button bx:click=shout{ field: @doc:msg }>!</button></article>"#,
		);
		reactive_html(&mut world, root)
			.xpect_contains("data-bx-verbs")
			.xpect_contains("entity.set_field(args.field, 'HEY');");
	}

	#[beet_core::test]
	fn injects_blob_and_runtime_into_head() {
		let mut world = ui_world();
		let root = build(&mut world, COUNTER_PAGE);
		let html = reactive_html(&mut world, root);
		// everything reactive lands inside <head>, before its close tag; the body
		// carries only the bound run.
		let head = html.split("</head>").next().unwrap().to_string();
		head.clone().xpect_contains("data-bx-blob");
		head.xpect_contains(
			"<script defer src=\"/js/reactivity.js\"></script>",
		);
		html.xpect_contains("<!--bx-ref=\"counter.count\"-->0<!--bx-end-->");
	}

	#[beet_core::test]
	fn auto_keeps_plain_page_clean() {
		let mut world = ui_world();
		let root = build(&mut world, PLAIN_PAGE);
		// no bindings: an Auto reactive render is byte-identical to the static one.
		reactive_html(&mut world, root).xpect_eq(static_html(&mut world, root));
	}

	#[beet_core::test]
	fn always_injects_runtime_on_plain_page() {
		let mut world = ui_world();
		let root = build(&mut world, PLAIN_PAGE);
		// Always ships the runtime even with no bindings (no blob, just the script).
		reactive_html_with(&mut world, root, InsertReactive::Always, false)
			.xpect_contains("<script defer src=\"/js/reactivity.js\"></script>")
			.xnot()
			.xpect_contains("data-bx-blob");
	}

	#[beet_core::test]
	fn inline_embeds_runtime_source() {
		let mut world = ui_world();
		let root = build(&mut world, COUNTER_PAGE);
		// inline mode embeds the runtime source instead of referencing it by URL.
		reactive_html_with(&mut world, root, InsertReactive::Auto, true)
			.xpect_contains("EntityMut")
			.xnot()
			.xpect_contains("src=\"/js/reactivity.js\"");
	}

	/// Transcluded content carries its own document on the content tree's root,
	/// *above* the transclusion entry. The walk must resolve it from the entry's
	/// ancestors, so the entry element tops the document and the blob ships.
	#[beet_core::test]
	fn document_above_transclusion_entry_is_marked() {
		let mut world = ui_world();
		// content tree: the document on the root, a bound run two levels below
		let content_root = world
			.spawn((
				Element::new("article"),
				Document::new(val!({ "count": 0 })),
			))
			.id();
		let inner =
			world.spawn((ChildOf(content_root), Element::new("p"))).id();
		world.spawn((ChildOf(inner), Value::default(), FieldRef::new("count")));
		world.update_local();
		// a holder transcludes `inner` (below the document) by reference
		let holder = world.spawn(Portal::new(inner)).id();

		reactive_html(&mut world, holder)
			// the entry element tops the document region
			.xpect_contains("data-bx-doc=\"d0\"")
			// and the document state ships even though it lives above the entry
			.xpect_contains("data-bx-blob>{\"d0\":{\"count\":0}}")
			.xpect_contains("<!--bx-ref=\"count\"-->0<!--bx-end-->");
	}

	/// A binding into a props store is server-only (the store is never shipped),
	/// so it must not emit a reactive run that the client would clear on load.
	#[beet_core::test]
	fn prop_binding_into_props_store_is_not_reactive() {
		let mut world = ui_world();
		let doc = world
			.spawn((Element::new("div"), Document::new(val!({ "count": 0 }))))
			.id();
		// a props store (server-only) holding the materialized prop
		let store = world
			.spawn((
				ChildOf(doc),
				Document::new(val!({ "title": "Hi" })),
				PropsDocument,
			))
			.id();
		let heading = world.spawn((ChildOf(store), Element::new("h2"))).id();
		// `@prop:title` reads the props store; `@doc:count` reads the user document
		world.spawn((
			ChildOf(heading),
			Value::default(),
			FieldRef::new("title").with_document(DocumentPath::Props),
		));
		let para = world.spawn((ChildOf(doc), Element::new("p"))).id();
		world.spawn((ChildOf(para), Value::default(), FieldRef::new("count")));
		world.update_local();

		reactive_html(&mut world, doc)
			// the `@doc` run is reactive
			.xpect_contains("<!--bx-ref=\"count\"-->")
			// the `@prop` run is not (the props store is never shipped)
			.xnot()
			.xpect_contains("bx-ref=\"title\"");
	}

	/// A flat counter exercising every default verb plus a custom one, the fixture
	/// the Playwright check drives.
	#[cfg(not(target_arch = "wasm32"))]
	const PLAYWRIGHT_COUNTER: &str = r#"<html><head><title>counter</title></head><body><article>
<p id="count">count is {@doc:count=0}</p>
<p id="flag">flag is {@doc:flag=false}</p>
<p id="status">status is {@doc:status="pending"}</p>
<button id="inc" bx:click=increment{ field: @doc:count, amount: 2 }>+</button>
<button id="dec" bx:click=decrement{ field: @doc:count }>-</button>
<button id="tog" bx:click=toggle{ field: @doc:flag }>toggle</button>
<button id="set" bx:click=set{ field: @doc:status, value: "done" }>set</button>
<button id="dbl" bx:click=double{ field: @doc:count }>double</button>
</article></body></html>"#;

	/// Render the [`PLAYWRIGHT_COUNTER`] reactively (inline runtime, so the file is
	/// self-contained for `file://`) with a custom `double` verb, and write it to
	/// `target/playwright/counter.html`, the input the Playwright check drives.
	///
	/// Run: `cargo test -p beet_ui --lib writes_playwright_fixture`, then
	/// `node crates/beet_ui/src/render/html/reactivity.playwright.cjs`.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	fn writes_playwright_fixture() {
		let mut world = ui_world();
		// a custom app verb with its JS twin: proves the `data-bx-verbs` seam end to
		// end in a real browser.
		world.resource_mut::<VerbRegistry>().insert(
			"double",
			VerbSchema::new().binding("field"),
			Some("entity.set_field(args.field, (Number(entity.get_field(args.field)) || 0) * 2);"),
			|_, _| {},
		);
		let root = build(&mut world, PLAYWRIGHT_COUNTER);
		// several `@doc` fields initialize the same not-yet-created document in one
		// pass, racing its deferred creation so only the last survives; seed the
		// full initial state so the blob ships every value (the SSR text is already
		// correct, so this just lines the blob up with it: no hydration flash).
		world
			.run_system_once(
				|mut docs: Query<&mut Document, Without<PropsDocument>>| {
					for mut doc in docs.iter_mut() {
						doc.0 = val!({ "count": 0, "flag": false, "status": "pending" });
					}
				},
			)
			.unwrap();
		let html =
			reactive_html_with(&mut world, root, InsertReactive::Auto, true);
		// the fixture carries the wire format, the custom verb twin, and the runtime
		html.clone()
			.xpect_contains("data-bx-doc=")
			.xpect_contains("data-bx-blob")
			.xpect_contains("class EntityMut")
			.xpect_contains("Number(entity.get_field(args.field))");
		let path =
			fs_ext::workspace_root().join("target/playwright/counter.html");
		fs_ext::write(&path, &html).unwrap();
	}

	/// The JSON payload of a `<script ..MARKER>..</script>` emitted by the render.
	#[cfg(target_arch = "wasm32")]
	fn extract_script(html: &str, marker: &str) -> String {
		let open = format!("{marker}>");
		let start = html.find(&open).unwrap() + open.len();
		let end = html[start..].find("</script>").unwrap();
		html[start..start + end].to_string()
	}

	/// End-to-end under deno (no DOM): render the counter reactively to get the
	/// real blob and the emitted default verb twins, evaluate [`REACTIVITY_JS`],
	/// install those verbs, drive the non-DOM `EntityMut` through every default
	/// verb, and marshal the resulting store state + recorded mutations back to
	/// Rust. This exercises the actually-shipped JS, not a stand-in.
	///
	/// Run: `beet test --target=wasm32-unknown-unknown -p beet_ui --lib`.
	#[cfg(target_arch = "wasm32")]
	#[beet_core::test]
	fn runtime_drives_emitted_default_verbs() {
		let mut world = ui_world();
		let root = build(&mut world, COUNTER);
		let html = reactive_html(&mut world, root);
		// the actual wire-format payloads the page ships
		let blob = extract_script(&html, "data-bx-blob");
		let verbs = extract_script(&html, "data-bx-verbs");

		// evaluate the real runtime: its IIFE publishes the api on `globalThis.beet`
		js_sys::eval(REACTIVITY_JS).unwrap();
		// a driver that installs the emitted verbs and drives the non-DOM EntityMut
		// through each default verb, recording every store mutation
		let driver = format!(
			r#"(function() {{
				const beet = globalThis.beet;
				const store = beet.createStore({blob});
				beet.installVerbs({verbs});
				const mutations = [];
				store.subscribe((docId, path) => mutations.push(path));
				const entity = new beet.EntityMut(store, "d0", null, null);
				beet.verbs.increment(entity, {{ field: "counter.count", amount: 2 }});
				beet.verbs.increment(entity, {{ field: "counter.count" }});
				beet.verbs.decrement(entity, {{ field: "counter.count" }});
				beet.verbs.set(entity, {{ field: "flag", value: false }});
				beet.verbs.toggle(entity, {{ field: "flag" }});
				return JSON.stringify({{
					count: store.get("d0", "counter.count"),
					flag: store.get("d0", "flag"),
					mutations: mutations.length,
				}});
			}})()"#
		);
		let result = js_sys::eval(&driver).unwrap().as_string().unwrap();
		// 0 +2 +1 -1 = 2; flag false -> toggled true; five recorded mutations
		result
			.xpect_contains("\"count\":2")
			.xpect_contains("\"flag\":true")
			.xpect_contains("\"mutations\":5");
	}
}
