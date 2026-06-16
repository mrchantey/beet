+++
title = "beet_ui"
+++

# beet_ui

`beet_ui` represents structured documents as ECS data. It borrows the vocabulary of XML, nodes, elements and attributes, but treats a tree as a living thing in a Bevy world rather than a static file.

Trees are written with the `rsx!` macro, using familiar markup:

```rust
# use beet_ui::prelude::*;
rsx! {
	<p style="color: pink">"Hello world!"</p>
};
```

which expands to plain entities and components:

```rust
# use beet_ui::prelude::*;
# use beet_core::prelude::*;
World::new().spawn((
	Element::new("p"),
	related!(Attributes[
		(Attribute::new("style"), Value::new("color: pink"))
	]),
	children![ Value::new("Hello world!") ]
));
```

Leaning into ECS loosens XML's rules in useful ways. A single node can hold both an `Element` and a `Value`, the `Value` type covers both text nodes and attribute values, a value node can have children of its own, and an empty node is simply a fragment. The tree is composable in the way components are.

Reusable widgets are `#[template]` function components used as capitalized tags like `<Button/>`. The same tree feeds two kinds of integration: parsers diff a byte stream against an entity, and renderers walk the tree to produce output. The renderers are target-agnostic, so one scene becomes an HTML page or a character-cell terminal view without rewriting it. Styling follows the same principle, semantic `Classes` resolved by a rule set instead of stringly-typed CSS, which keeps the contract between widgets and styles in the type system. See the [Design](/docs/design) section for the system built on top.
