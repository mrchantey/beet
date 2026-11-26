//! [`Lowering`]: the walk state, the keys it mints and the nodes that are no
//! element.
use super::binding::ReservedRef;
use super::literal::*;
use crate::body::KeySentinels;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::TypePath;
use bevy::reflect::TypeRegistry;

/// How a document's roots become its first entity.
#[derive(Debug, Default, Clone, PartialEq)]
pub(in crate::bsx) enum LowerRoot {
	/// A single root element is the root itself, else a bare root holds the
	/// roots as its children.
	#[default]
	Document,
	/// Always a root holding these components, every node its child: a
	/// format whose root is the document rather than an element of it, ie
	/// markdown.
	Container(EntityBody),
}

/// One document being lowered.
pub(super) struct Lowering<'a> {
	/// The directives and types the document lowers against.
	pub parsers: &'a DocParsers,
	pub types: &'a TypeRegistry,
	/// The body so far, its entities in document order.
	pub body: SceneBody,
	/// Where each of them was written.
	pub spans: SourceMap,
	/// Every name a `bx:ref` declares.
	refs: HashSet<SmolStr>,
	/// The document's root key, what `BuildRoot` and `SnippetRoot` name.
	root: EntityKey,
	/// Whether the markup lowers as written, resolving nothing.
	plain: bool,
}

impl BsxParser {
	/// Lower `nodes`, rewriting their directives first.
	pub(in crate::bsx) fn lower_nodes(
		mut nodes: Vec<BsxNode>,
		root: LowerRoot,
		parsers: &DocParsers,
	) -> Result<ParsedDocument> {
		BsxDirectives::of(parsers).rewrite(&mut nodes)?;
		let types = parsers.types().read();
		Lowering::lower(nodes, root, parsers, &types)
	}
}

impl<'a> Lowering<'a> {
	/// Lower `nodes`, whose directives are already rewritten.
	pub fn lower(
		nodes: Vec<BsxNode>,
		root: LowerRoot,
		parsers: &'a DocParsers,
		types: &'a TypeRegistry,
	) -> Result<ParsedDocument> {
		let mut refs = HashSet::default();
		Self::collect_refs(&nodes, &mut refs)?;
		let mut lowering = Self {
			parsers,
			types,
			body: default(),
			spans: default(),
			refs,
			root: EntityKey::ROOT,
			plain: false,
		};
		match (root, BsxNode::single_root(&nodes)) {
			(LowerRoot::Document, Some(BsxNode::Element(element)))
				if element.tag != "slot" =>
			{
				if let Some(name) = &element.key {
					lowering.root = EntityKey::parse(name)?;
				}
				let root = lowering.root.clone();
				lowering.push(root.clone(), default(), &element.span)?;
				lowering
					.element(element, root, None)
					.map_err(|error| element.span.locate(error))?;
			}
			(LowerRoot::Document, _) => {
				// a bare root reaches over everything it holds
				let span = match (nodes.first(), nodes.last()) {
					(Some(first), Some(last)) => first.span().to(last.span()),
					_ => default(),
				};
				lowering.push(EntityKey::ROOT, default(), &span)?;
				lowering.children(&nodes, &EntityKey::ROOT, None)?;
			}
			(LowerRoot::Container(components), _) => {
				lowering.push(EntityKey::ROOT, components, &default())?;
				lowering.children(&nodes, &EntityKey::ROOT, None)?;
			}
		}
		lowering.finish().xok()
	}

	/// Lower `nodes` as the markup they are, under a bare root.
	pub fn lower_plain(
		nodes: &[BsxNode],
		parsers: &'a DocParsers,
		types: &'a TypeRegistry,
	) -> Result<ParsedDocument> {
		let mut lowering = Self {
			parsers,
			types,
			body: default(),
			spans: default(),
			refs: default(),
			root: EntityKey::ROOT,
			plain: true,
		};
		lowering.push(EntityKey::ROOT, default(), &default())?;
		lowering.children(nodes, &EntityKey::ROOT, None)?;
		lowering.finish().xok()
	}

	/// The document lowered.
	fn finish(self) -> ParsedDocument {
		ParsedDocument {
			body: self.body,
			spans: self.spans,
		}
	}

	/// Collect every `bx:ref` name, refusing a reserved one and a name two
	/// elements declare.
	fn collect_refs(nodes: &[BsxNode], refs: &mut HashSet<SmolStr>) -> Result {
		for node in nodes {
			let BsxNode::Element(element) = node else {
				continue;
			};
			if let Some(name) = &element.key {
				if ReservedRef::parse(name).is_some() {
					return Err(element.span.locate(bevyhow!(
						"`bx:ref=\"{name}\"` shadows a reserved ref name, reserved: {:?}",
						ReservedRef::NAMES
					)));
				}
				if !refs.insert(name.clone()) {
					return Err(element.span.locate(bevyhow!(
						"`bx:ref=\"{name}\"` is declared twice, and a ref names \
						one entity of its document"
					)));
				}
			}
			Self::collect_refs(&element.children, refs)?;
		}
		Ok(())
	}

	/// The key a `$name` references: the entity its `bx:ref` names, which may
	/// come later, or the root for a name reserved for it.
	pub fn reference(&self, name: &str) -> Result<EntityKey> {
		match ReservedRef::parse(name) {
			Some(ReservedRef::BuildRoot | ReservedRef::SnippetRoot) => {
				self.root.clone().xok()
			}
			Some(ReservedRef::PageRoot | ReservedRef::Router) => bevybail!(
				"`${name}` is resolved by a binding alone, ie \
				`@entity:{name}::Type.field`"
			),
			None => {
				if !self.refs.contains(name) {
					warn!(
						"`${name}` names no `bx:ref` of its document: the reference binds to nothing"
					);
				}
				EntityKey::parse(name)
			}
		}
	}

	/// Add an entity written at `span`, after every one added so far.
	pub fn push(
		&mut self,
		key: EntityKey,
		components: EntityBody,
		span: &SourceSpan,
	) -> Result {
		if self.body.entities.contains_key(&key) {
			bevybail!(
				"two entities are keyed `{key}`: a `bx:ref` names one entity \
				and never a structural key"
			);
		}
		if let Some(span) = span.get() {
			self.spans.insert(key.clone(), span.clone());
		}
		self.body.entities.insert(key, components);
		Ok(())
	}

	/// The components of the entity keyed `key`.
	pub fn entity(&mut self, key: &EntityKey) -> &mut EntityBody {
		self.body.entities.entry(key.clone()).or_default()
	}

	/// Add `component` to the entity keyed `key`.
	pub fn insert<T: TypePath + Serialize>(
		&mut self,
		key: &EntityKey,
		component: &T,
	) -> Result {
		self.entity(key).insert_typed(component)
	}

	/// Add the component `build` returns to the entity keyed `key`, each
	/// entity it asks for by key written as that key.
	pub fn insert_linked<T: TypePath + Serialize>(
		&mut self,
		key: &EntityKey,
		build: impl FnOnce(&mut KeySentinels) -> T,
	) -> Result {
		let mut keys = KeySentinels::default();
		let component = build(&mut keys);
		let value = keys.keyed(Value::from_serde(&component)?)?;
		self.entity(key).insert(T::type_path(), value);
		Ok(())
	}

	/// Lower `nodes` as the children of `parent`, each routed to the default
	/// slot of `slotted` (a template tag or an [`Instance`]) unless it routes
	/// itself.
	pub fn children(
		&mut self,
		nodes: &[BsxNode],
		parent: &EntityKey,
		slotted: Option<&str>,
	) -> Result {
		let mut counts = HashMap::<SmolStr, usize>::default();
		for node in nodes {
			self.child(node, parent, slotted, &mut counts)
				.map_err(|error| node.span().locate(error))?;
		}
		Ok(())
	}

	/// Lower one of `parent`'s children.
	fn child(
		&mut self,
		node: &BsxNode,
		parent: &EntityKey,
		slotted: Option<&str>,
		counts: &mut HashMap<SmolStr, usize>,
	) -> Result {
		// an embedded document is read first, as its root's tag keys it
		let embedded = match node {
			BsxNode::Embedded(embedded) => Some(self.read_embedded(embedded)?),
			_ => None,
		};
		let key = match &embedded {
			Some(document) => Self::embedded_key(document, parent, counts)?,
			None => Self::child_key(node, parent, counts)?,
		};
		let mut components = EntityBody::default();
		components.insert(ChildOf::type_path(), parent.reference());
		self.push(key.clone(), components, node.span())?;
		match (node, embedded) {
			(_, Some(document)) => self.embedded(&document, &key)?,
			(BsxNode::Element(element), _) if self.plain => {
				self.plain_element(element, &key)?
			}
			(BsxNode::Element(element), _) => {
				self.element(element, key.clone(), Some(parent))?
			}
			// an expression has no plain form
			(BsxNode::Expr(_), _) if self.plain => {}
			(node, None) => self.leaf(node, &key)?,
		}
		// a child that never renders, a `<script {OnEvent}>` a
		// `bx:onclick` rewrote to, is behaviour of the tag itself and
		// stays on it, never slotted
		if slotted.is_some() {
			let components = self.entity(&key);
			if !components.contains_typed::<SlotChild>()
				&& !components.contains_typed::<RunScript>()
				&& !components.contains_typed::<OnEvent>()
			{
				components.insert_typed(&SlotChild::new())?;
			}
		}
		Ok(())
	}

	/// The key of `node` under `parent`: the name its `bx:ref` gives it, else
	/// minted from its tag and its index among siblings of that tag.
	fn child_key(
		node: &BsxNode,
		parent: &EntityKey,
		counts: &mut HashMap<SmolStr, usize>,
	) -> Result<EntityKey> {
		let tag = match node {
			BsxNode::Element(element) => {
				if let Some(name) = &element.key {
					return EntityKey::parse(name);
				}
				element.tag.as_str()
			}
			BsxNode::Text(_) | BsxNode::Expr(_) => "#text",
			BsxNode::Comment(_) => "#comment",
			BsxNode::Doctype(_) => "#doctype",
			BsxNode::CData(_) => "#cdata",
			BsxNode::ProcessingInstruction(_) => "#pi",
			// keyed by its root's tag once it is read, see `child`
			BsxNode::Embedded(_) => "#document",
		};
		Self::counted_key(tag, parent, counts)
	}

	/// The key of the next child of `parent` tagged `tag`.
	pub fn counted_key(
		tag: &str,
		parent: &EntityKey,
		counts: &mut HashMap<SmolStr, usize>,
	) -> Result<EntityKey> {
		let count = counts.entry(tag.into()).or_default();
		let key = EntityKey::parse(&format!("{parent}/{tag}.{count}"));
		*count += 1;
		key
	}

	/// Add an attribute entity named `name` to the entity keyed `owner`, its
	/// components `components`, written at `span`.
	pub fn attribute(
		&mut self,
		owner: &EntityKey,
		name: &str,
		components: EntityBody,
		span: &SourceSpan,
	) -> Result<EntityKey> {
		let mut key = EntityKey::parse(&format!("{owner}/@{name}"))?;
		// an attribute written twice, ie `class` once by hand and once by a
		// directive, is two entities
		let mut index = 0;
		while self.body.entities.contains_key(&key) {
			index += 1;
			key = EntityKey::parse(&format!("{owner}/@{name}.{index}"))?;
		}
		let mut attribute = EntityBody::default();
		attribute.insert(AttributeOf::type_path(), owner.reference());
		for (name, value) in Map::from(components) {
			attribute.insert(name, value);
		}
		self.push(key.clone(), attribute, span)?;
		key.xok()
	}

	/// Lower a text, expr, comment, doctype, CDATA or processing instruction
	/// node onto the entity keyed `key`.
	fn leaf(&mut self, node: &BsxNode, key: &EntityKey) -> Result {
		match node {
			BsxNode::Text(text) => {
				self.insert(key, &Value::Str(text.as_str().into()))
			}
			BsxNode::Expr(expr) => self.value_expr(expr, key, None),
			BsxNode::Comment(content) => {
				self.insert(key, &Comment::new(content.value.clone()))
			}
			BsxNode::Doctype(value) => {
				self.insert(key, &Doctype::new(value.value.clone()))
			}
			BsxNode::CData(content) => {
				self.insert(key, &CData::new(content.value.clone()))
			}
			BsxNode::ProcessingInstruction(content) => self.insert(
				key,
				&ProcessingInstruction::new(content.value.clone()),
			),
			BsxNode::Element(_) | BsxNode::Embedded(_) => {
				unreachable!("an element or an embedded document is no leaf")
			}
		}
	}

	/// Lower a text or attribute position value onto the entity keyed `key`:
	/// a literal is its [`Value`], a binding its source's components, an
	/// `@comp` binding the element keyed `element` in attribute position and
	/// the entity itself in text position.
	pub fn value_expr(
		&mut self,
		expr: &ValueExpr,
		key: &EntityKey,
		element: Option<&EntityKey>,
	) -> Result {
		match expr {
			ValueExpr::Literal(literal) => {
				let value = literal_to_value(literal)?;
				self.insert(key, &value)
			}
			ValueExpr::Binding(binding) => self.binding(binding, key, element),
			ValueExpr::EntityRef(_) => {
				bevybail!(
					"`$name` entity references are not valid in text position"
				)
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A tuple-struct component with an `Entity` field, the `StoreRef` shape a
	/// declaration is named by.
	#[derive(Component, Reflect, MapEntities, Clone, Debug, PartialEq)]
	#[reflect(Component, MapEntities, Default)]
	struct Bound(#[entities] Entity);

	impl Default for Bound {
		fn default() -> Self { Self(Entity::PLACEHOLDER) }
	}

	/// The named-field twin of [`Bound`], the shape a consumer of SEVERAL
	/// declarations names them all in, one field per role.
	#[derive(Component, Reflect, MapEntities, Clone, Debug, PartialEq)]
	#[reflect(Component, MapEntities, Default)]
	struct BoundPair {
		#[entities]
		first: Entity,
		#[entities]
		second: Entity,
	}

	impl Default for BoundPair {
		fn default() -> Self {
			Self {
				first: Entity::PLACEHOLDER,
				second: Entity::PLACEHOLDER,
			}
		}
	}

	/// A verdict whose failing variant carries `()`, the shape of
	/// beet_action's `Outcome`.
	#[derive(Reflect, Clone, Debug, Default, PartialEq)]
	enum Verdict {
		#[default]
		Pass,
		Fail(()),
	}

	/// A leaf holding a verdict, the shape of `EndWith<Outcome>`.
	#[derive(Component, Reflect, Clone, Debug, Default, PartialEq)]
	#[reflect(Component, Default)]
	struct Leaf {
		value: Verdict,
	}

	/// A reference its type's `map_entities` never visits, which would bind to
	/// nothing.
	#[derive(Component, Reflect)]
	#[reflect(Component, Default)]
	struct Unmapped(Entity);

	impl Default for Unmapped {
		fn default() -> Self { Self(Entity::PLACEHOLDER) }
	}

	#[beet_core::test]
	fn an_unmapped_reference_is_refused() {
		try_build(r#"<main><a bx:ref="there"/><b {Unmapped($there)}/></main>"#)
			.unwrap_err()
			.to_string()
			.xpect_contains("#[entities]");
		try_build("<b {Unmapped}/>").unwrap();
	}

	/// A serde of its own, and a default minted per value, as a thread's id is.
	#[derive(Component, Reflect, serde::Serialize, serde::Deserialize)]
	#[reflect(Component, Default, Serialize, Deserialize)]
	struct Identified {
		id: SmolStr,
		name: SmolStr,
	}

	impl Default for Identified {
		fn default() -> Self {
			Self {
				id: "minted".into(),
				name: default(),
			}
		}
	}

	/// A document holds the fields it authored and nothing of the default's,
	/// whatever serde the type has.
	#[beet_core::test]
	fn a_default_is_never_written_down() {
		let (world, root) = build(r#"<Identified name="a"/>"#);
		world
			.get::<Identified>(root)
			.unwrap()
			.id
			.as_str()
			.xpect_eq("minted");
		BsxParser::body(&world, r#"<Identified name="a"/>"#)
			.unwrap()
			.entities[&EntityKey::ROOT]
			.get(Identified::type_path())
			.unwrap()
			.clone()
			.xpect_eq(value!({ "name": "a" }));
	}

	/// An option whose default is a value.
	#[derive(Component, Reflect)]
	#[reflect(Component, Default)]
	struct Optional {
		name: Option<SmolStr>,
	}

	impl Default for Optional {
		fn default() -> Self {
			Self {
				name: Some("default".into()),
			}
		}
	}

	/// The `None` literal is the option's own, never the string it spells.
	#[beet_core::test]
	fn a_none_literal_overrides_a_default() {
		for (markup, name) in [
			("<Optional/>", Some("default")),
			("<Optional name={None}/>", None),
			(r#"<Optional name="None"/>"#, Some("None")),
		] {
			let (world, root) = build(markup);
			world
				.get::<Optional>(root)
				.unwrap()
				.name
				.as_deref()
				.xpect_eq(name);
		}
	}

	/// A bare variant name builds a tuple variant carrying `()`: `value=Fail`,
	/// a unit enum literal, once missed the variant in `from_reflect` and kept
	/// the leaf's default, a pass.
	#[beet_core::test]
	fn an_attribute_spells_a_unit_carrying_variant() {
		let (mut world, _) = build("<Leaf value=Fail/>");
		world
			.query::<&Leaf>()
			.single(&world)
			.unwrap()
			.value
			.clone()
			.xpect_eq(Verdict::Fail(()));
		// a name the enum lacks is refused rather than defaulted
		try_build("<Leaf value=Fial/>")
			.unwrap_err()
			.to_string()
			.xpect_contains("`Fial` names no unit variant");
	}

	/// Build `markup` into a world registering `PackageConfig` and [`Bound`],
	/// returning the world and the document's root.
	fn build(markup: &str) -> (World, Entity) { try_build(markup).unwrap() }

	/// Build `markup`, returning the world and the built root, or the build
	/// error.
	fn try_build(markup: &str) -> Result<(World, Entity)> {
		let mut world = (ScenePlugin, DocumentPlugin).into_world();
		{
			let registry = world.resource_mut::<AppTypeRegistry>();
			let mut registry = registry.write();
			registry.register::<PackageConfig>();
			registry.register::<Bound>();
			registry.register::<BoundPair>();
			registry.register::<RequireCfg>();
			registry.register::<Leaf>();
			registry.register::<Unmapped>();
			registry.register::<Identified>();
			registry.register::<Optional>();
		}
		let body = BsxParser::body(&world, markup)?;
		let root = world.spawn_template(body)?.id();
		world.flush();
		Ok((world, root))
	}

	/// The entities `markup` builds, root included.
	fn entity_count(markup: &str) -> usize {
		let (mut world, root) = build(markup);
		world.with_state::<Query<&Children>, _>(|children| {
			children.iter_descendants_inclusive(root).count()
		})
	}

	/// A directive rewrites in a binary lacking the type it names, which then
	/// holds that type as any unregistered name: core has no `InlineStyle`.
	#[beet_core::test]
	fn a_lean_binary_holds_an_inline_style() {
		let (world, root) = build(r#"<div bx:style="display=Flex"/>"#);
		world
			.entity(root)
			.get::<Unregistered>()
			.unwrap()
			.clone()
			.xpect_eq(Unregistered::new(
				"InlineStyle",
				value!({ "declarations": "display=Flex" }),
			));
	}

	/// Structure is universal: an unresolvable tag is an inert entity holding
	/// the component it named, attributes and all, and its children build
	/// under it exactly as they would in a binary that linked the type.
	#[beet_core::test]
	fn unregistered_tag_is_inert_and_builds_children() {
		let (world, root) = build(
			r#"<NotRegistered name="ada" count=3 on><PackageConfig/></NotRegistered>"#,
		);
		let inert = root;
		world
			.entity(inert)
			.get::<Unregistered>()
			.unwrap()
			.clone()
			.xpect_eq(Unregistered::new(
				"NotRegistered",
				value!({ "name": "ada", "count": 3, "on": true }),
			));
		// the child built, and the resource declaration it carries applied.
		world
			.entity(inert)
			.get::<Children>()
			.unwrap()
			.len()
			.xpect_eq(1);
		// the same shape a registered tag would build: root, tag, child.
		entity_count("<NotRegistered><PackageConfig/></NotRegistered>")
			.xpect_eq(entity_count("<div><PackageConfig/></div>"));
	}

	/// A string attribute fills a field, the shape an entry declares its cfg
	/// requirement in.
	#[beet_core::test]
	fn attribute_fills_a_field() {
		let (world, root) = build(
			r#"<RequireCfg cfg="feature:ml && feature:beet_esp/alvik"/>"#,
		);
		let host = root;
		world
			.entity(host)
			.get::<RequireCfg>()
			.unwrap()
			.xpect_eq(RequireCfg::new("feature:ml && feature:beet_esp/alvik"));
	}

	/// An attribute naming no field of a component is an error naming both,
	/// since a component has nowhere to forward it, while a directive still
	/// passes.
	///
	/// REGRESSION: the attribute rode the patch untyped and the apply over the
	/// default dropped it, so `<R2BucketBlock retain_days=30/>` against a block
	/// with no such field rendered a bucket with no lock and no warning.
	#[beet_core::test]
	fn an_attribute_naming_no_field_is_refused() {
		try_build(r#"<RequireCfg cfg="feature:ml" cfgs="feature:extra"/>"#)
			.unwrap_err()
			.to_string()
			.xpect_contains("`<RequireCfg>` has no field `cfgs`")
			.xpect_contains("its fields are cfg");
		try_build(r#"<RequireCfg cfg="feature:ml" bx:ref="one"/>"#).unwrap();
	}

	/// A `$name` reference into a formerly-gated region resolves to the real
	/// inert entity, not a never-built placeholder.
	#[beet_core::test]
	fn ref_into_an_unregistered_subtree_resolves() {
		let (world, root) = build(
			r#"<div><NotRegistered bx:ref="target"/><span {Bound($target)}/></div>"#,
		);
		let host = root;
		let children = world.entity(host).get::<Children>().unwrap();
		let (target, consumer) = (children[0], children[1]);
		world.entity(target).contains::<Unregistered>().xpect_true();
		world
			.entity(consumer)
			.get::<Bound>()
			.unwrap()
			.0
			.xpect_eq(target);
	}

	/// A `$name` reference fills a NAMED struct field, so a consumer of several
	/// declarations names every one of them in one component with every role
	/// spelled, rather than one relationship per store.
	#[beet_core::test]
	fn refs_fill_named_fields() {
		let (world, root) = build(
			r#"<div><span bx:ref="a"/><span bx:ref="b"/><span {BoundPair{first: $a, second: $b}}/></div>"#,
		);
		let host = root;
		let children = world.entity(host).get::<Children>().unwrap();
		let (first, second, consumer) = (children[0], children[1], children[2]);
		world
			.entity(consumer)
			.get::<BoundPair>()
			.unwrap()
			.xpect_eq(BoundPair { first, second });
	}

	/// A spread naming nothing registered is left off and held with its
	/// literal, beside whatever else the entity holds, where a strict caller
	/// finds the typo it might be.
	#[beet_core::test]
	fn an_unregistered_spread_is_held() {
		let (mut world, div) =
			build(r#"<div {(Mispelt{size: 2}, Tagged("x"), Bare)}/>"#);
		world
			.entity(div)
			.get::<Element>()
			.unwrap()
			.tag()
			.xpect_eq("div");
		world
			.entity(div)
			.get::<Unregistered>()
			.unwrap()
			.clone()
			.xpect_eq(Unregistered::from_iter([
				("Mispelt", value!({ "size": 2 })),
				("Tagged", Value::str("x")),
				("Bare", Value::map()),
			]));
		world
			.with_state::<UnregisteredQuery, _>(|query| query.refuse(div))
			.unwrap_err()
			.to_string()
			.xpect_contains("`Bare`, `Mispelt`, `Tagged`");
	}

	/// The keys `markup` lowers to, in document order.
	fn keys(markup: &str) -> Vec<String> {
		BsxParser::body(&ScenePlugin::world(), markup)
			.unwrap()
			.entities
			.keys()
			.map(|key| key.to_string())
			.collect()
	}

	/// A key is minted from structure, an attribute's name its own segment.
	#[beet_core::test]
	fn mints_keys_from_structure() {
		keys(
			r#"<main class="a"><p>one</p><p>two</p>{@doc:three}<!-- c --></main>"#,
		)
		.xpect_eq(vec![
			"0",
			"0/@class",
			"0/p.0",
			"0/p.0/#text.0",
			"0/p.1",
			"0/p.1/#text.0",
			"0/#text.0",
			"0/#comment.0",
		]);
	}

	/// Inserting an element leaves every sibling of another tag keyed as it
	/// was, and a `bx:ref` keeps its name, and its descendants theirs, wherever
	/// the element moves.
	#[beet_core::test]
	fn keys_survive_an_insertion_and_a_move() {
		let before = keys("<main><p><b/></p><p/></main>");
		let after = keys("<main><div/><p><b/></p><p/></main>");
		before.iter().all(|key| after.contains(key)).xpect_true();

		let named = |markup: &str| {
			keys(markup)
				.into_iter()
				.filter(|key| key.starts_with("card"))
				.collect::<Vec<_>>()
		};
		named(r#"<main><p bx:ref="card"><b/></p><div/></main>"#)
			.xpect_eq(vec!["card", "card/b.0"])
			.xpect_eq(named(
				r#"<main><div><i/><p bx:ref="card"><b/></p></div></main>"#,
			));
	}

	/// A document with several roots is a bare root holding each; a comment
	/// beside a single root documents the file.
	#[beet_core::test]
	fn several_roots_are_children_of_a_bare_one() {
		keys("<!doctype html><html/>").xpect_eq(vec![
			"0",
			"0/#doctype.0",
			"0/html.0",
		]);
		keys("<!-- a page -->\n<html/>\n").xpect_eq(vec!["0"]);
		keys("text").xpect_eq(vec!["0", "0/#text.0"]);
	}

	/// A name two elements declare is refused, and so is a reserved one.
	#[beet_core::test]
	fn a_ref_names_one_entity() {
		try_build(r#"<main><p bx:ref="a"/><p bx:ref="a"/></main>"#)
			.unwrap_err()
			.to_string()
			.xpect_contains("`bx:ref=\"a\"` is declared twice");
		try_build(r#"<main bx:ref="PageRoot"/>"#)
			.unwrap_err()
			.to_string()
			.xpect_contains("shadows a reserved ref name");
	}

	/// A body holds what a document says as data: its types by path, its
	/// fields as authored and no more, a reference its key.
	#[beet_core::test]
	fn lowers_to_data() {
		let mut world = (ScenePlugin, DocumentPlugin).into_world();
		world
			.resource_mut::<AppTypeRegistry>()
			.write()
			.register::<Bound>();
		let body = BsxParser::body(
			&world,
			r#"<main><p bx:ref="to"/><p {Bound($to)} {Missing{size: 2}}/></main>"#,
		)
		.unwrap();
		let entity = &body.entities[&EntityKey::parse("0/p.0").unwrap()];
		entity
			.get(<Bound as bevy::reflect::TypePath>::type_path())
			.unwrap()
			.xpect_eq(Value::str("to"));
		entity
			.get("Missing")
			.unwrap()
			.xpect_eq(value!({ "size": 2 }));
		// and round trips through bytes to the same spawn
		#[cfg(feature = "json")]
		DocParsers::of(&world)
			.body(&body.to_bytes(MediaType::Json).unwrap())
			.unwrap()
			.xpect_eq(body);
	}

	/// The resources of a spread or tag, the one patch over a live one.
	#[derive(Debug, Default, Resource, Reflect)]
	#[reflect(Default, Resource)]
	struct Score {
		value: i64,
		kept: i64,
	}

	/// A resource named in markup patches the live one's named fields and
	/// never lands on its entity; an absent one is made over its default.
	#[beet_core::test]
	fn a_resource_is_patched_where_it_is_declared() {
		let world = || {
			let world = ScenePlugin::world();
			world
				.resource::<AppTypeRegistry>()
				.write()
				.register::<Score>();
			world
		};
		for markup in
			["<div {Score{value:42}}/>", "<div><Score value=42/></div>"]
		{
			let mut live = world();
			live.insert_resource(Score { value: 1, kept: 2 });
			let body = BsxParser::body(&live, markup).unwrap();
			let root = live.spawn_template(body.clone()).unwrap().id();
			live.resource::<Score>().value.xpect_eq(42);
			live.resource::<Score>().kept.xpect_eq(2);
			live.entity(root).get::<Score>().xpect_none();

			let mut absent = world();
			absent.spawn_template(body).unwrap();
			absent.resource::<Score>().value.xpect_eq(42);
		}
	}

	/// A template building an element named by its prop.
	#[derive(Default, Clone, Reflect)]
	#[reflect(Default)]
	struct Link {
		href: String,
	}
	subtree_template!(Link);
	impl GetTemplateSchema for Link {}
	impl bevy::ecs::template::Template for Link {
		type Output = ();
		fn build_template(
			&self,
			cx: &mut bevy::ecs::template::TemplateContext,
		) -> Result<()> {
			let root = cx.entity.id();
			cx.entity.insert(Element::new("a"));
			let href = self.href.clone();
			cx.entity.world_scope(|world| {
				for (key, value) in
					[("href", href.as_str()), ("class", "btn"), ("rel", "me")]
				{
					world.spawn((
						AttributeOf::new(root),
						Attribute::new(key),
						Value::str(value),
					));
				}
				world.spawn((SlotTarget::new(), ChildOf(root)));
			});
			OK
		}
		fn clone_template(&self) -> Self { self.clone() }
	}

	/// A template is an invocation in the body, expanded as it spawns, its
	/// children slotted and an attribute beyond its props forwarded to the
	/// root element it built.
	#[beet_core::test]
	fn a_template_is_its_invocation() {
		let mut app = App::new();
		app.add_plugins((ScenePlugin, DocumentPlugin))
			.install_types::<Link>();
		let world = app.world_mut();
		let body = BsxParser::body(
			world,
			r#"<Link href="/docs" class="wide" rel="next"><b/><script {OnEvent{event:"click"}}>x</script></Link>"#,
		)
		.unwrap();
		body.entities[&EntityKey::ROOT]
			.get(<Link as bevy::reflect::TypePath>::type_path())
			.unwrap()
			.xpect_eq(value!({ "href": "/docs" }));
		let root = world.spawn_template(body).unwrap().id();
		let mut attributes = world
			.entity(root)
			.get::<Attributes>()
			.unwrap()
			.iter()
			.map(|entity| {
				format!(
					"{}={}",
					world.get::<Attribute>(entity).unwrap().as_str(),
					world.get::<Value>(entity).unwrap()
				)
			})
			.collect::<Vec<_>>();
		attributes.sort();
		attributes.xpect_eq(vec![
			"class=btn",
			"class=wide",
			"href=/docs",
			"rel=next",
		]);
		// the child filled the slot, and the handler stayed on its host
		let children = world.entity(root).get::<Children>().unwrap().to_vec();
		children
			.iter()
			.filter(|child| world.get::<Element>(**child).is_some())
			.count()
			.xpect_eq(2);
		children
			.iter()
			.any(|child| world.get::<OnEvent>(*child).is_some())
			.xpect_true();
	}

	/// A binding has nothing to sync with in a template's prop, so it is
	/// refused rather than dropped.
	#[beet_core::test]
	fn a_bound_template_prop_is_refused() {
		let mut app = App::new();
		app.add_plugins((ScenePlugin, DocumentPlugin))
			.install_types::<Link>();
		BsxParser::body(app.world(), "<Link href=@doc:url/>")
			.unwrap_err()
			.to_string()
			.xpect_contains("a template prop takes a literal");
	}

	/// An instance's literal props are its own and a bound one a binding
	/// related to the use site, the prop seeded until its source lands.
	#[beet_core::test]
	fn an_instance_is_a_reference() {
		let world = (ScenePlugin, DocumentPlugin).into_world();
		let body = BsxParser::body(
			&world,
			r#"<main><Instance document="./Card" props={{title:"Intro", count:@doc:count=3}}><p/></Instance></main>"#,
		)
		.unwrap();
		let entity =
			|key: &str| &body.entities[&EntityKey::parse(key).unwrap()];
		entity("0/Instance.0").get_typed::<Value>().xpect_err();
		entity("0/Instance.0")
			.get(<Instance as bevy::reflect::TypePath>::type_path())
			.unwrap()
			.xpect_eq(value!({
				"document": "./Card",
				"props": { "title": "Intro", "count": 3 }
			}));
		let bound = entity("0/Instance.0/@count");
		// its sink names the use site, as the key a body holds for an entity
		bound
			.get(<FieldRef as bevy::reflect::TypePath>::type_path())
			.unwrap()
			.as_map()
			.unwrap()
			.get("document")
			.unwrap()
			.xpect_eq(value!({ "Entity": "0/Instance.0" }));
		bound.contains_typed::<SourceFieldRef>().xpect_true();
		// its children are its slot children
		entity("0/Instance.0/p.0")
			.contains_typed::<SlotChild>()
			.xpect_true();
	}

	/// Another document is named by an `Instance`, never by a tag.
	#[beet_core::test]
	fn a_template_tag_is_refused_with_its_instance() {
		try_build(r#"<Template src="a.bsx"/>"#)
			.unwrap_err()
			.to_string()
			.xpect_contains("<Instance document");
	}

	/// A relative reference resolves from the document it was written in,
	/// wherever the component holding it is.
	#[beet_core::test]
	fn paths_resolve_as_a_body_reflects() {
		let mut world = (ScenePlugin, DocumentPlugin).into_world();
		let body = BsxParser::body(&world, r#"<Instance document="../Card"/>"#)
			.unwrap();
		let template = DynamicTemplate::reflect_at(
			&world,
			&body,
			&RelPath::new("routes/blog/post.bsx"),
		)
		.unwrap();
		let root = world.spawn_template(template).unwrap().id();
		world
			.get::<Instance>(root)
			.unwrap()
			.document
			.as_str()
			.xpect_eq("routes/Card");
	}

	/// A prefab in markup is held whole, the templates in it expanded and
	/// slotted first.
	#[beet_core::test]
	fn a_template_element_is_held() {
		let (world, root) = build("<ul><template><li>x</li></template></ul>");
		let prefab = world.entity(root).get::<Children>().unwrap()[0];
		world.entity(prefab).contains::<Prefab>().xpect_true();
		let item = world.entity(prefab).get::<Children>().unwrap()[0];
		world.entity(item).contains::<Held>().xpect_true();
	}

	/// A markup-declared `<PackageConfig/>` patches only its named fields over
	/// [`PackageConfig::default`]: set fields override, unset fields keep the
	/// defaults (and the optionals stay `None`).
	#[beet_core::test]
	fn markup_patches_over_defaults() {
		let mut world = (ScenePlugin, DocumentPlugin).into_world();
		world
			.resource_mut::<AppTypeRegistry>()
			.write()
			.register::<PackageConfig>();
		let body =
			BsxParser::body(&world, r#"<PackageConfig title="Patched"/>"#)
				.unwrap();
		world.spawn_template(body).unwrap();

		let config = world.resource::<PackageConfig>();
		// the set field overrides the default
		config.title.as_str().xpect_eq("Patched");
		// unset fields keep their defaults
		config
			.description
			.as_str()
			.xpect_eq("An app built with beet");
		config.app_name.as_str().xpect_eq("beet-app");
		// version keeps the default since the markup did not set it
		config.version.as_str().xpect_eq("0.0.1");
	}
}

#[cfg(test)]
mod group_test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Build `markup` into a world that registers the membership types.
	fn build(markup: &str) -> (World, Entity) {
		let mut world =
			(ScenePlugin, DocumentPlugin, MinimalTypesPlugin).into_world();
		let body = BsxParser::body(&world, markup).unwrap();
		let root = world.spawn_template(body).unwrap().id();
		world.flush();
		(world, root)
	}

	/// A `$ref` may point forward, so a member declared above the group it
	/// joins enrolls all the same, in the order the file declares it.
	#[beet_core::test]
	fn a_forward_reference_enrolls_in_file_order() {
		let (world, root) = build(
			r#"<div>
				<span {MemberOf($later)}/>
				<em {MemberOf($later)}/>
				<Group bx:ref="later"/>
			</div>"#,
		);
		let children = world.entity(root).get::<Children>().unwrap();
		let (first, second, group) = (children[0], children[1], children[2]);
		world
			.entity(group)
			.get::<Members>()
			.unwrap()
			.iter()
			.collect::<Vec<_>>()
			.xpect_eq(vec![first, second]);
	}

	/// A `$ref` naming something that never became a group fails when the
	/// document settles, rather than when the group is later run.
	#[beet_core::test]
	#[should_panic(expected = "has members but is not a `Group`")]
	fn enrolling_into_a_non_group_fails() {
		build(r#"<div><span {MemberOf($target)}/><em bx:ref="target"/></div>"#);
	}
}
