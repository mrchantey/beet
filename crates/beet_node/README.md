# beet_node

Agnostic structured data representations in Bevy ECS.


beet_node borrows the concepts of `Nodes`, `Elements` and `Attributes` from xml, however beet prioritizes runtime behavior over static formats.

Unlike xml beet leans into the composable nature of ECS. 
- A node may contain both an [`Element`] and a [`Value`].
- The [`Value`] type is used for both an xml text node and an attribute value.
- A [`Value`] node may have children
- Empty nodes are considered fragments

```rust
# use beet_node::prelude::*;
# use beet_core::prelude::*;

let my_node = World::new().spawn((
	Element::new("p"),
	// attribute nodes are attached via the Attributes relation
	related!(Attributes[
		(Attribute::new("style"), Value::new("color: pink"))
	]),
	// descendents may be an Element or Value node
	children![
		Value::new("Hello world!")
	]
));
```

## Integrations

Various formats are provided out of the box, and come in one of two flavors:

### Parsers

Parsers accept a stream of bytes and diff them against an entity.


### Renderers

Renderers walk a tree of nodes and perform some action like appending a html buffer or initializing a persistent ui.
