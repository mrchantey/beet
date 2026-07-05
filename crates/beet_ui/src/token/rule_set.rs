use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;
use std::collections::VecDeque;

/// Global store of style [`Rule`]s.
///
/// Holds an ordered list of matching rules plus a single `:root` default rule.
/// The default rule is the **lowest-priority fallback**: the cascade only
/// consults it (via [`RuleSetQuery`]) once the matching rules and the ancestor
/// walk find nothing, so a matching rule like `.dark-scheme` always overrides a
/// value baked into `:root`. Among the matching rules, earlier entries win ties
/// (they're ordered most-specific first).
#[derive(Debug, Clone, Reflect, Resource)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RuleSet {
	/// The `:root` rule — the lowest-priority fallback, kept out of `rules` so
	/// it never shadows a matching rule.
	default_rule: Rule,
	/// Ordered matching rules; earlier rules win ties.
	rules: VecDeque<Rule>,
	/// Inline rules are only declared once. Calling [`Self::try_insert_inline`]
	/// with a rule whose selector matches one of these does nothing.
	registered_inline: HashSet<Selector>,
}

/// By default, the rule set is initialized with an empty `:root` rule.
impl Default for RuleSet {
	fn default() -> Self { Self::new(default()) }
}

impl RuleSet {
	pub fn new(default_rule: Rule) -> Self {
		Self {
			default_rule,
			rules: VecDeque::new(),
			registered_inline: default(),
		}
	}

	/// Attempt to register an inline rule. If a rule with the same selector
	/// has already been registered this does nothing and returns `false`,
	/// otherwise the rule is inserted and `true` is returned.
	pub fn try_insert_inline(&mut self, rule: Rule) -> bool {
		if self.registered_inline.contains(rule.selector()) {
			return false;
		}
		self.registered_inline.insert(rule.selector().clone());
		self.insert_rule(rule);
		true
	}

	/// Add a new rule, merging with the last added when both its selector and
	/// `@media` gate match. The media check keeps a screen/terminal-gated rule
	/// from folding its declarations into an adjacent ungated rule with the same
	/// selector (eg `.sidebar` + a screen-only `.sidebar` width), which would
	/// strip the gate and leak the value to every target.
	pub fn insert_rule(&mut self, rule: Rule) {
		if let Some(last) = self.rules.back_mut()
			&& last.selector() == rule.selector()
			&& last.media() == rule.media()
		{
			last.push_declarations(rule);
		} else {
			self.rules.push_back(rule);
		}
	}
	pub fn with_rule(mut self, rule: Rule) -> Self {
		self.insert_rule(rule);
		self
	}
	/// Inserts multiple rules.
	pub fn with_rules(mut self, rules: impl IntoIterator<Item = Rule>) -> Self {
		for rule in rules {
			self.insert_rule(rule);
		}
		self
	}

	/// Find the first rule matching `Selector::Entity(entity)` that contains `key`.
	pub fn find_entity_rule_mut(
		&mut self,
		entity: Entity,
		key: &TokenKey,
	) -> Option<&mut Rule> {
		self.rules.iter_mut().find(|r| {
			r.selector() == &Selector::Entity(entity) && r.contains_key(key)
		})
	}

	/// Iterates the matching rules in insertion order, excluding the `:root`
	/// default rule.
	pub fn rules(&self) -> impl Iterator<Item = &Rule> { self.rules.iter() }

	/// Iterates every rule for serialization — the `:root` default first, then
	/// the matching rules.
	pub fn iter(&self) -> impl Iterator<Item = &Rule> {
		core::iter::once(&self.default_rule).chain(self.rules.iter())
	}

	/// The `:root` default rule — the lowest-priority cascade fallback.
	pub fn default_rule(&self) -> &Rule { &self.default_rule }
	/// Mutable access to the `:root` default rule.
	pub fn default_rule_mut(&mut self) -> &mut Rule { &mut self.default_rule }
	pub fn insert(
		&mut self,
		key: impl Into<Token>,
		value: impl Into<TokenValue>,
	) -> Result<&mut Self> {
		self.default_rule_mut().insert(key, value)?;
		self.xok()
	}
	fn with(
		mut self,
		key: impl Into<Token>,
		value: impl Into<TokenValue>,
	) -> Result<Self> {
		self.insert(key, value)?;
		self.xok()
	}

	pub fn with_token(
		self,
		key: impl Into<Token>,
		value: impl Into<Token>,
	) -> Result<Self> {
		self.with(key, value)
	}
	#[cfg(feature = "serde")]
	pub fn with_value(
		self,
		key: impl Into<Token>,
		value: impl Typed + Serialize,
	) -> Result<Self> {
		self.with(key, TypedValue::new(value)?)
	}
	#[cfg(feature = "serde")]
	#[track_caller]
	pub fn with_inline_value<T>(self, value: T) -> Result<Self>
	where
		T: Typed + Serialize,
	{
		let key = Token::new_inline(FieldSchema::of::<T>());
		self.with(key, TypedValue::new(value)?)
	}

	/// Extend with multiple rules, inserting each (merging when selectors match).
	pub fn extend_rules(
		&mut self,
		rules: impl IntoIterator<Item = Rule>,
	) -> &mut Self {
		for rule in rules {
			self.insert_rule(rule);
		}
		self
	}

	/// Indices (into [`Self::rules`]) of the rules whose selector matches `el`,
	/// in source order. Computed once per element by [`RuleSetQuery`] and reused
	/// across that element's ~30 property lookups, so the 228-rule selector scan
	/// runs once per element instead of once per property.
	///
	/// `@media`-gated rules apply per [`MediaQuery::applies_in_terminal`]: a
	/// `Terminal` rule always (the one query whose context is this cascade), a
	/// width-gated rule when the element's surface `viewport` sits at or below
	/// its breakpoint, and the web-only queries (print/screen/reduced-motion)
	/// never — those only affect CSS output.
	fn matching_rule_indices(
		&self,
		el: &ElementView,
		ancestors: &[ElementView],
		viewport: Option<MediaViewport>,
	) -> Vec<usize> {
		self.rules
			.iter()
			.enumerate()
			.filter(|(_, rule)| {
				rule.media()
					.is_none_or(|media| media.applies_in_terminal(viewport))
					&& rule.selector().matches(el, ancestors)
			})
			.map(|(index, _)| index)
			.collect()
	}

	/// Whether any registered rule is gated on a viewport-width query
	/// ([`MediaQuery::MaxWidth`]), ie whether the cascade must resolve each
	/// element's surface [`MediaViewport`] to match. Width-free rule sets (the
	/// common case) skip that walk, and `resolve_styles` only re-cascades on a
	/// surface resize when this holds.
	pub fn has_width_media(&self) -> bool {
		self.rules
			.iter()
			.any(|rule| matches!(rule.media(), Some(MediaQuery::MaxWidth(_))))
	}

	/// Whether any registered rule uses a combinator selector (`>` or
	/// descendant), ie whether the cascade must build the ancestor element chain
	/// to match. Combinator-free rule sets (the common case) skip that work.
	fn has_combinator_rules(&self) -> bool {
		self.rules
			.iter()
			.any(|rule| rule.selector().is_combinator_deep())
	}

	/// Pick the winning declaration for `key` among the pre-matched rules. The
	/// `:root` default rule is the lowest-priority fallback, applied by
	/// [`RuleSetQuery`] after the ancestor walk, so a matching rule (eg
	/// `.dark-scheme`) can override a `:root` default, mirroring CSS. The most
	/// specific matching rule wins (class beats tag); ties go to the later rule,
	/// mirroring CSS source order (and the serialized stylesheet) so a theme
	/// override appended after a user-agent default wins on both.
	fn cascade_in(
		&self,
		matched: &[usize],
		key: &Token,
	) -> Result<&TokenValue> {
		matched
			.iter()
			.filter_map(|&index| {
				let rule = &self.rules[index];
				rule.get(key)
					.ok()
					.map(|value| (rule.selector().specificity(), value))
			})
			.reduce(|best, next| if next.0 >= best.0 { next } else { best })
			.map(|(_, value)| value)
			.ok_or_else(|| bevyhow!("no matching rule for token `{key}`"))
	}
}

/// Within-pass cascade memo, owned by [`resolve_styles`] and fresh each pass so
/// no stale value leaks across frames. Two caches collapse the cost:
///
/// - `values`: `(Entity, Token)` -> resolved [`Value`] (`None` = no match). An
///   inherited token re-walked for every descendant becomes a single map hit
///   instead of another ancestor walk, the fix for the O(n²) inheritance blowup.
/// - `matched_rules`: nearest-element [`Entity`] -> indices of the rules whose
///   selector matches it. Resolving an element touches ~30 properties; without
///   this each would re-scan all rules, so this runs the selector scan once per
///   element instead of once per property.
#[derive(Default)]
pub struct CascadeMemo {
	values: HashMap<(Entity, Token), Option<Value>>,
	matched_rules: HashMap<Entity, Vec<usize>>,
	/// query [`Entity`] -> its nearest-ancestor element entity, so the
	/// `get_in_ancestors` walk + [`ElementView`] build runs once per entity
	/// rather than once per resolved property.
	nearest_element: HashMap<Entity, Entity>,
	/// Whether the rule set has any combinator (`>`/descendant) rule, resolved
	/// once per pass. `None` until first computed; gates the ancestor-chain walk
	/// so combinator-free rule sets pay nothing for it.
	has_combinators: Option<bool>,
	/// Whether the rule set has any width-gated (`MaxWidth`) media rule,
	/// resolved once per pass; gates the surface-viewport walk the same way
	/// `has_combinators` gates the ancestor chain.
	has_width_media: Option<bool>,
}

#[derive(SystemParam)]
pub struct RuleSetQuery<'w, 's> {
	rule_set: ResMut<'w, RuleSet>,
	ancestors: Query<'w, 's, &'static ChildOf>,
	_children: Query<'w, 's, &'static Children>,
	// the [`Portal`] reverse edge, so the inherited cascade crosses transclusion
	// boundaries: content transcluded into a layout by reference has no `ChildOf`
	// edge to the layout, so inheritance (eg the color scheme) continues from the
	// holder that renders it in place.
	render_refs: Query<'w, 's, &'static PortalOf>,
	// the surface viewport width-gated media rules resolve against, found by
	// walking the same Portal-aware parent chain inheritance uses.
	viewports: Query<'w, 's, &'static MediaViewport>,
	element_query: ElementQuery<'w, 's>,
}

impl RuleSetQuery<'_, '_> {
	/// Resolve `token` for `entity`, memoizing the result in `memo`. The cache
	/// collapses the inheritance ancestor walk (and `:root` fallback) so each
	/// `(entity, token)` is cascaded once per pass.
	pub fn resolve<T>(
		&self,
		entity: Entity,
		token: T,
		memo: &mut CascadeMemo,
	) -> Result<T::Value>
	where
		T: TypedToken + Into<Token>,
		T::Value: DeserializeOwned,
	{
		self.resolve_untyped(entity, &token.into(), memo)
			.and_then(|value| value.into_serde::<T::Value>())
	}
	pub fn resolve_untyped(
		&self,
		entity: Entity,
		token: &Token,
		memo: &mut CascadeMemo,
	) -> Result<Value> {
		// inheritance re-walks the same `(ancestor, token)` for every descendant,
		// so a cache hit here is what turns the cascade from O(n²) back to O(n).
		let key = (entity, token.clone());
		if let Some(cached) = memo.values.get(&key) {
			return cached.clone().ok_or_else(|| {
				bevyhow!("no matching rule for token `{token}`")
			});
		}
		let resolved = self.resolve_untyped_uncached(entity, token, memo);
		memo.values.insert(key, resolved.as_ref().ok().cloned());
		resolved
	}

	fn resolve_untyped_uncached(
		&self,
		entity: Entity,
		token: &Token,
		memo: &mut CascadeMemo,
	) -> Result<Value> {
		match self.cascade(entity, token, memo) {
			Ok(TokenValue::Value(value)) =>
			// mapped directly to value, ie background-color: green
			{
				value.value().clone().xok()
			}
			Ok(TokenValue::Token(next)) => {
				// points to another token ie background-color: primary
				let next = next.clone();
				self.resolve_untyped(entity, &next, memo)
			}
			Err(err) => {
				// inherited tokens search ancestors before the root fallback
				if token.is_inherited()
					&& let Some(ancestor) = self.parent(entity)
				{
					self.resolve_untyped(ancestor, token, memo)
				} else {
					// fall back to the `:root` default declarations
					self.resolve_default(entity, token, memo).map_err(|_| err)
				}
			}
		}
	}

	/// The cascade parent of `entity`. A transcluded entity (the target of a
	/// [`Portal`]) inherits from the holder that renders it in place, not from
	/// its original [`ChildOf`] spawn location — so the cascade (eg the color
	/// scheme) crosses the transclusion boundary. Otherwise the `ChildOf` parent.
	fn parent(&self, entity: Entity) -> Option<Entity> {
		Portal::visual_parent(&self.ancestors, &self.render_refs, entity)
	}

	/// The [`MediaViewport`] of the surface `entity` renders into: the nearest
	/// self-or-ancestor carrying one, walking the same Portal-aware
	/// [`parent`](Self::parent) chain inheritance uses, so transcluded content
	/// (eg a live page under a buffer host's slot) resolves the surface that
	/// renders it. `None` when no surface exists (eg building static HTML
	/// server-side), which skips width-gated rules.
	fn surface_viewport(&self, entity: Entity) -> Option<MediaViewport> {
		let mut current = entity;
		loop {
			if let Ok(viewport) = self.viewports.get(current) {
				return Some(*viewport);
			}
			match self.parent(current) {
				// a self-referential edge would loop; a malformed graph is a clean stop.
				Some(parent) if parent != current => current = parent,
				_ => return None,
			}
		}
	}

	/// See [`RuleSet::has_width_media`].
	pub fn has_width_media(&self) -> bool { self.rule_set.has_width_media() }

	/// The ancestor element views of `entity`, nearest-first, for evaluating the
	/// combinator selectors (`>`, descendant). Walks the same Portal-aware
	/// [`parent`](Self::parent) chain inheritance uses, so content transcluded
	/// into `<main>` is seen as its child, and skips non-element (text/fragment)
	/// nodes so `main > *` reads the same tree the HTML serializer flattens to.
	fn ancestor_elements(&self, entity: Entity) -> Vec<ElementView<'_>> {
		let mut ancestors = Vec::new();
		let mut current = entity;
		while let Some(parent) = self.parent(current) {
			// a self-referential edge would loop; a malformed graph is a clean stop.
			if parent == current {
				break;
			}
			current = parent;
			if let Ok(view) = self.element_query.get(parent) {
				ancestors.push(view);
			}
		}
		ancestors
	}

	/// Resolves `token` against the `:root` default rule — the lowest-priority
	/// fallback consulted once the cascade and ancestor walk find nothing.
	fn resolve_default(
		&self,
		entity: Entity,
		token: &Token,
		memo: &mut CascadeMemo,
	) -> Result<Value> {
		match self.rule_set.default_rule().get(token)? {
			TokenValue::Value(value) => value.value().clone().xok(),
			TokenValue::Token(next) => {
				let next = next.clone();
				self.resolve_untyped(entity, &next, memo)
			}
		}
	}
	pub fn cascade<'a>(
		&'a self,
		entity: Entity,
		token: &Token,
		memo: &mut CascadeMemo,
	) -> Result<&'a TokenValue> {
		// fast path: once an entity's nearest element and that element's matched
		// rules are cached, skip the `get_in_ancestors` walk and `ElementView`
		// build entirely (the common case across an entity's ~30 properties).
		if let Some(element) = memo.nearest_element.get(&entity)
			&& let Some(matched) = memo.matched_rules.get(element)
		{
			return self.rule_set.cascade_in(matched, token);
		}
		// cold path: resolve the nearest ancestor element (handling text and
		// fragment nodes) and the rules matching it, caching both.
		let el = self.element_query.get_in_ancestors(entity)?;
		memo.nearest_element.insert(entity, el.entity);
		// only a combinator rule (`main > *`) needs the ancestor chain, and only
		// a width-gated rule needs the surface viewport, so resolve each lazily
		// and only when the rule set calls for it.
		let needs_ancestors =
			*memo.has_combinators.get_or_insert_with(|| {
				self.rule_set.has_combinator_rules()
			});
		let needs_viewport = *memo
			.has_width_media
			.get_or_insert_with(|| self.rule_set.has_width_media());
		let matched = memo.matched_rules.entry(el.entity).or_insert_with(|| {
			let ancestors = needs_ancestors
				.then(|| self.ancestor_elements(el.entity))
				.unwrap_or_default();
			let viewport = needs_viewport
				.then(|| self.surface_viewport(el.entity))
				.flatten();
			self.rule_set.matching_rule_indices(&el, &ancestors, viewport)
		});
		self.rule_set.cascade_in(matched, token)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	token!(Foo, u32);
	token!(Bar, u32);

	#[beet_core::test]
	fn cascade() {
		let mut world = World::new();
		// `Bar`'s value lives in the `:root` default rule (the lowest-priority
		// fallback); `Foo` points at `Bar` from a matching rule.
		world.insert_resource(
			RuleSet::default().with_value(Bar, 3u32).unwrap().with_rule(
				Rule::new()
					.with_selector(Selector::Any)
					.with_token(Foo, Bar)
					.unwrap(),
			),
		);
		let mut entity = world.spawn(rsx! { <div/> });

		// a matching (non-default) rule is found directly by `cascade`
		entity
			.with_state::<RuleSetQuery, _>(|entity, query| {
				query.cascade(entity, &Foo.into(), &mut default()).cloned()
			})
			.unwrap()
			.xpect_eq(TokenValue::token(Bar));

		// `Bar` lives only in the `:root` default rule, which `cascade` excludes ...
		entity
			.with_state::<RuleSetQuery, _>(|entity, query| {
				query.cascade(entity, &Bar.into(), &mut default()).is_err()
			})
			.xpect_true();

		// ... but resolution falls back to it, following the token chain
		entity
			.with_state::<RuleSetQuery, _>(|entity, query| {
				query.resolve_untyped(entity, &Foo.into(), &mut default())
			})
			.unwrap()
			.xpect_eq(3u32.into());
	}

	/// The entity of the sole element with `tag`.
	fn tag_entity(world: &mut World, tag: &str) -> Entity {
		let mut query = world.query::<(Entity, &Element)>();
		query
			.iter(world)
			.find(|(_, element)| element.tag() == tag)
			.map(|(entity, _)| entity)
			.unwrap()
	}

	/// Whether the combinator rule setting `Foo` selects `entity`. Uses `cascade`
	/// (a direct selector match) rather than `resolve` so an inherited token's
	/// ancestor fallback can't be mistaken for a combinator match.
	fn selects(world: &mut World, entity: Entity) -> bool {
		world.with_state::<RuleSetQuery, _>(|query| {
			query.cascade(entity, &Foo.into(), &mut default()).is_ok()
		})
	}

	// the charcell cascade evaluates `main > *` against the real ancestor chain:
	// a direct child matches, a grandchild and main itself do not.
	#[beet_core::test]
	fn child_combinator_cascade() {
		let mut world = World::new();
		world.insert_resource(RuleSet::default().with_rule(
			Rule::new()
				.with_selector(Selector::child(
					Selector::tag("main"),
					Selector::Any,
				))
				.with_value(Foo, 1u32),
		));
		world.spawn(rsx! { <main><span><em/></span></main> });
		let (span, em, main) = (
			tag_entity(&mut world, "span"),
			tag_entity(&mut world, "em"),
			tag_entity(&mut world, "main"),
		);
		selects(&mut world, span).xpect_true();
		selects(&mut world, em).xpect_false();
		selects(&mut world, main).xpect_false();
	}

	/// A rule gating `Foo` behind `max-width: 1024px`.
	fn max_width_rule() -> Rule {
		Rule::new()
			.with_media(MediaQuery::MaxWidth(1024))
			.with_selector(Selector::Any)
			.with_value(Foo, 1u32)
	}

	/// Spawn `<div/>` under a surface `width_px` wide, returning the div.
	fn div_under_viewport(world: &mut World, width_px: f32) -> Entity {
		let surface = world
			.spawn((MediaViewport::new(width_px, 768.), children![rsx! { <div/> }]))
			.id();
		world.entity(surface).get::<Children>().unwrap()[0]
	}

	// a `MaxWidth`-gated rule applies at or below its breakpoint (inclusive,
	// like CSS `max-width`), and is skipped above it or when no surface
	// viewport exists at all (eg building static HTML, where the browser
	// evaluates the serialized `@media` instead).
	#[beet_core::test]
	fn max_width_cascade() {
		let mut world = World::new();
		world.insert_resource(RuleSet::default().with_rule(max_width_rule()));
		let narrow = div_under_viewport(&mut world, 640.);
		let exact = div_under_viewport(&mut world, 1024.);
		let wide = div_under_viewport(&mut world, 1600.);
		let unhosted = world.spawn(rsx! { <div/> }).id();
		selects(&mut world, narrow).xpect_true();
		selects(&mut world, exact).xpect_true();
		selects(&mut world, wide).xpect_false();
		selects(&mut world, unhosted).xpect_false();
	}

	// transcluded content (eg a live page under a buffer host's `Portal` slot)
	// resolves the surface that renders it across the transclusion boundary.
	#[beet_core::test]
	fn max_width_crosses_portals() {
		let mut world = World::new();
		world.insert_resource(RuleSet::default().with_rule(max_width_rule()));
		let content = world.spawn(rsx! { <div/> }).id();
		world.spawn((
			MediaViewport::new(640., 768.),
			children![Portal::new(content)],
		));
		selects(&mut world, content).xpect_true();
	}

	// the descendant combinator `main em` matches at any depth, unlike `>`.
	#[beet_core::test]
	fn descendant_combinator_cascade() {
		let mut world = World::new();
		world.insert_resource(RuleSet::default().with_rule(
			Rule::new()
				.with_selector(Selector::descendant(
					Selector::tag("main"),
					Selector::tag("em"),
				))
				.with_value(Foo, 1u32),
		));
		world.spawn(rsx! { <main><span><em/></span></main> });
		let (em, span) =
			(tag_entity(&mut world, "em"), tag_entity(&mut world, "span"));
		selects(&mut world, em).xpect_true();
		selects(&mut world, span).xpect_false();
	}
}
