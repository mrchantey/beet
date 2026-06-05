+++
title = "beet_ui"
+++

# beet_ui

`beet_ui` represents structured documents as ECS data. It borrows the vocabulary of XML, nodes, elements and attributes, but treats a tree as a living thing in a Bevy world rather than a static file.

Leaning into ECS loosens XML's rules in useful ways. A single node can hold both an `Element` and a `Value`, the `Value` type covers both text nodes and attribute values, a value node can have children of its own, and an empty node is simply a fragment. The tree is composable in the way components are.

```rust
# use beet_ui::prelude::*;
# use beet_core::prelude::*;
let my_node = World::new().spawn((
	Element::new("p"),
	related!(Attributes[
		(Attribute::new("style"), Value::new("color: pink"))
	]),
	children![ Value::new("Hello world!") ]
));
```

You rarely build trees by hand. The `rsx!` macro writes them with familiar markup, and reusable widgets are `#[scene]` function components used as capitalized tags like `<Button/>`. The same tree feeds two kinds of integration: parsers diff a byte stream against an entity, and renderers walk the tree to produce output. Crucially the renderers are target-agnostic, so one scene becomes an HTML page or a character-cell terminal view without rewriting it. Styling follows the same principle, semantic `Classes` resolved by a rule set instead of stringly-typed CSS, which keeps the contract between widgets and styles in the type system. See the [Design](/docs/design) section for the system built on top.
