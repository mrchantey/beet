//! The thin-client reactivity wire format: the annotations [`HtmlRenderer`] emits
//! in reactive mode ([`HtmlRenderer::reactive`]) so a browser runtime can hydrate
//! and drive a page with no WASM. This module is the single source of truth for
//! that format; the JavaScript runtime (`ReactivityScript`) mirrors it.
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
//!   app-supplied JS verbs (omitted when none), installed alongside the runtime's
//!   built-in defaults.
//!
//! Only `@doc`/`@prop` document state is in scope; `@res`/`@comp` (reflect) are
//! server-rendered only, a later WASM concern.

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemState;

/// Reactivity annotations collected from the world before a reactive render, plus
/// the walk-time state for emitting them. See the [module docs](self) for the
/// wire format.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Reactive {
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
	/// App-supplied JS verbs as `(name, source)`.
	verbs: Vec<(SmolStr, String)>,
	/// Walk state: the governing document of each enclosing visited element.
	element_doc_stack: Vec<Option<usize>>,
}

impl Reactive {
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

	/// Walk the render tree (transparent through [`RenderRef`]), assigning each
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
		// a RenderRef holder is transparent: descend into the referenced entity in
		// the same document, mirroring the node walker.
		if let Some(target) = world.get::<RenderRef>(entity).map(RenderRef::target) {
			self.walk_docs(world, target, current, doc_ids, blob);
			return;
		}
		let mut doc = current;
		if world.get::<Document>(entity).is_some()
			&& world.get::<PropsDocument>(entity).is_none()
		{
			let id = *doc_ids.entry(entity).or_insert_with(|| {
				let id = blob.len();
				blob.push(world.get::<Document>(entity).unwrap().0.clone());
				id
			});
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

	/// Record the dotted path of every bound text node and attribute entity, keyed
	/// by entity for the visit-time lookup.
	fn collect_paths(&mut self, world: &mut World) {
		let mut state = SystemState::<
			Query<(
				Entity,
				&ResolvedFieldPath,
				Option<&Element>,
				Option<&AttributeOf>,
			)>,
		>::new(world);
		let query = state.get(world).expect("infallible binding query");
		for (entity, resolved, element, attribute_of) in query.iter() {
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
				let call = render_verb_call(binding, entity, &resolver, &scopes);
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
		let emit = governing
			.filter(|id| self.referenced.contains(id) && Some(*id) != enclosing);
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

	/// The `<script>` registering app-supplied JS verbs, `None` when there are
	/// none (the default set lives in the runtime, not the page).
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
			format!("{name}: {}", render_verb_arg(arg, entity, resolver, scopes))
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
				BindingSource::Res => return format!("@res:{}", expr.field_path),
				BindingSource::Comp => return format!("@comp:{}", expr.field_path),
			};
			let doc_entity = resolver.entity(entity, &document);
			let resolved =
				scopes.resolved_path(entity, &expr.field_path, Some(doc_entity));
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

/// Escape `<` so an embedded value can never close the host `<script>` element.
fn escape_script_json(json: &str) -> String { json.replace('<', "\\u003c") }

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
			.xpect_contains("bx:click=\"decrement{ field: @doc:counter.count }\"");
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
	fn defaults_emit_no_verbs_blob() {
		let mut world = ui_world();
		let root = build(&mut world, COUNTER);
		// the four default verbs are hand-written in the runtime, never emitted.
		reactive_html(&mut world, root)
			.xnot()
			.xpect_contains("data-bx-verbs");
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
}
