//! Pre-render hook that strips the back-of-card notes from a stack's cards.
//!
//! A card's source is its visible front (the content), a markdown thematic break
//! (`---`, lowered to a top-level `<hr>`), then the back-of-card notes that must
//! not render (the HyperCard "back of the card", eg speaker notes). [`card_notes`]
//! is a tree transform in the shared [`PostParseTree`] schedule (the same
//! chokepoint as
//! [`apply_syntax_highlighting`](beet_ui::prelude::apply_syntax_highlighting)),
//! so it runs for both the one-shot HTML render and the live TUI page build: it
//! finds the first top-level `<hr>` of each card and despawns it and every
//! following sibling, leaving the front content untouched.
//!
//! Gated on a [`CardDeck`] router being present in the world, so an `<hr>` in
//! ordinary beet content stays a horizontal rule. The gate is world-level (not an
//! ancestor walk) because per-request render content is spawned as a detached
//! root (see [`spawn_render_step`](crate::prelude::pure_route)), so it has no
//! router ancestor to walk to; a presented deck builds a single site, so "a deck
//! router exists" means "this render is a card". Registered by
//! [`CardStackPlugin`](crate::prelude::CardStackPlugin); it only runs when a deck
//! is present, never for general content.

use beet_core::prelude::*;

/// The synthetic wrapper a markdown HTML block parses into (eg an `.mdx` card
/// whose body mixes markdown with a `<TitleLayout>` tag): its children are the
/// card's top-level nodes, so it is transparent for "top-level" detection.
const HTML_BLOCK: &str = "__html_block";

/// System: remove each card's back-of-card notes, ie the first top-level `<hr>`
/// and every sibling after it. Runs only while a [`CardDeck`] router is present
/// (via [`run_if`]), so non-deck hosts keep their `<hr>` rules.
///
/// "Top-level" means a direct child of the card's content root: a fragment with
/// no [`Element`] of its own, or the synthetic [`HTML_BLOCK`] wrapper a markdown
/// HTML block parses into. A nested `<hr>` under a real content element (eg a
/// `<div>`) keeps an element parent and is left alone.
///
/// Idempotent: once the notes are gone the card has no top-level `<hr>`, so a
/// re-run (the live TUI repaints every frame) is a no-op.
pub fn card_notes(
	mut commands: Commands,
	elements: Query<(Entity, &Element)>,
	children: Query<&Children>,
	parents: Query<&ChildOf>,
) {
	// collect the content roots whose first top-level `<hr>` opens a notes block,
	// deduped: many top-level `<hr>`s can share a root, but only the first strips.
	let mut roots = HashSet::<Entity>::default();
	for (hr, _) in elements.iter().filter(|(_, el)| el.tag() == "hr") {
		// a top-level `<hr>` sits directly under the content root: a fragment with
		// no Element, or the transparent markdown HTML-block wrapper. A nested rule
		// under any other element keeps that element parent, so skip it.
		let Ok(root) = parents.get(hr).map(|parent| parent.parent()) else {
			continue;
		};
		let root_is_content = match elements.get(root) {
			Ok((_, el)) => el.tag() == HTML_BLOCK,
			Err(_) => true,
		};
		if !root_is_content {
			continue;
		}
		roots.insert(root);
	}

	// from each root, despawn the first top-level `<hr>` and every sibling after.
	for root in roots {
		let Ok(siblings) = children.get(root) else {
			continue;
		};
		let mut stripping = false;
		for sibling in siblings.iter() {
			if !stripping
				&& elements.get(sibling).is_ok_and(|(_, el)| el.tag() == "hr")
			{
				stripping = true;
			}
			if stripping {
				commands.entity(sibling).despawn();
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::HTML_BLOCK;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Run [`card_notes`] through its real `run_if` gate once against `world`: it
	/// strips only when a [`CardDeck`] is present, the same gate [`CardStackPlugin`]
	/// registers, so the tests exercise the gated behaviour.
	fn strip(world: &mut World) {
		let mut schedule = Schedule::default();
		schedule.add_systems(card_notes.run_if(any_with_component::<CardDeck>));
		schedule.run(world);
		world.flush();
	}

	/// The ordered child tags of `root`, the card's surviving top-level nodes.
	fn child_tags(world: &mut World, root: Entity) -> Vec<String> {
		world
			.entity(root)
			.get::<Children>()
			.map(|children| {
				children
					.iter()
					.filter_map(|child| world.entity(child).get::<Element>())
					.map(|el| el.tag().to_string())
					.collect()
			})
			.unwrap_or_default()
	}

	/// A `content`/`<hr>`/`notes` card tree: a content root (a fragment with no
	/// [`Element`]) holding a leading `<div>`, then a top-level `<hr>`, then the
	/// notes paragraph, modelling a parsed card. Returns the content root.
	fn card(world: &mut World) -> Entity {
		world
			.spawn(children![
				(Element::new("div"), children![(
					Element::new("p"),
					children![Value::str("content")]
				)]),
				Element::new("hr"),
				(Element::new("p"), children![Value::str("speaker notes")]),
			])
			.id()
	}

	/// A card keeps only its front content: the first top-level `<hr>` and the
	/// back-of-card notes after it are stripped while a [`CardDeck`] router is
	/// present. The card is a detached root (no router ancestor), mirroring the
	/// per-request render tree, so this proves the gate is world-level, not an
	/// ancestor walk.
	#[beet_core::test]
	fn strips_notes_from_card() {
		let mut world = World::new();
		let root = card(&mut world);
		// a deck router elsewhere in the world (not an ancestor of the card).
		world.spawn((Router, CardDeck));

		strip(&mut world);

		// only the leading content div survives; hr + notes are gone.
		child_tags(&mut world, root).xpect_eq(vec!["div".to_string()]);
	}

	/// With no [`CardDeck`] in the world the card keeps its `<hr>`: an `<hr>` is a
	/// legitimate horizontal rule outside a deck, so the `run_if` gate skips the
	/// system entirely.
	#[beet_core::test]
	fn keeps_hr_in_non_deck_page() {
		let mut world = World::new();
		let root = card(&mut world);
		// a plain router, no CardDeck marker, so the gate is false.
		world.spawn(Router);

		strip(&mut world);

		// the rule and the trailing paragraph both survive.
		child_tags(&mut world, root).xpect_eq(vec![
			"div".to_string(),
			"hr".to_string(),
			"p".to_string(),
		]);
	}

	/// A nested `<hr>` (a rule inside the content, not a top-level sibling) is kept
	/// even on a card: only the card's own top-level break opens notes.
	#[beet_core::test]
	fn keeps_nested_hr_in_card() {
		let mut world = World::new();
		// a deck router is present, so the gate runs the system.
		world.spawn((Router, CardDeck));
		// content root → <div> → <hr>: the rule's parent is an Element, so it is
		// nested, not a top-level node.
		let root = world
			.spawn(children![(Element::new("div"), children![Element::new(
				"hr"
			)])])
			.id();

		strip(&mut world);

		// the wrapping div (and its nested rule) survive untouched.
		child_tags(&mut world, root).xpect_eq(vec!["div".to_string()]);
		world
			.entity(root)
			.get::<Children>()
			.unwrap()
			.iter()
			.next()
			.map(|div| world.entity(div).get::<Children>().unwrap().len())
			.xpect_eq(Some(1));
	}

	/// A real `.mdx` card parses its body into an `__html_block` wrapper, so the
	/// top-level `<hr>` sits under that synthetic element rather than a bare
	/// fragment. The wrapper is transparent, so its notes still strip.
	#[beet_core::test]
	fn strips_notes_under_html_block_wrapper() {
		let mut world = World::new();
		world.spawn((Router, CardDeck));
		// content root → __html_block → [div, hr, notes], the structure an mdx card
		// (markdown body around a `<TitleLayout>`) renders into.
		let root = world
			.spawn(children![(Element::new(HTML_BLOCK), children![
				(Element::new("div"), children![(
					Element::new("p"),
					children![Value::str("content")]
				)]),
				Element::new("hr"),
				(Element::new("p"), children![Value::str("speaker notes")]),
			])])
			.id();

		strip(&mut world);

		// the wrapper keeps only its leading content div; hr + notes are gone.
		let block = world
			.entity(root)
			.get::<Children>()
			.unwrap()
			.iter()
			.next()
			.unwrap();
		child_tags(&mut world, block).xpect_eq(vec!["div".to_string()]);
	}
}
