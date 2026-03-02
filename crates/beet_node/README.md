# beet_node

Agnostic structured data representations in Bevy ECS.


beet_node borrows the concepts of `Nodes`, `Elements` and `Attributes` from xml.

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
