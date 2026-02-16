lets keep iterating on beet_stack!

Its time for a refactor! We will now formalize some concepts in beet_stack:

## Nodes

Content has so far been a grab-bag of as-needed html-like elements, lets rename this to `src/nodes` and formalize the concept. I'll tell you the general idea, but use more formal language and flesh out the module a bit more.

Beet Stack is an interface agnostic framework, in general we see paradigms like `<div><div<div>` 'structure as style' and css as an overreach on the side of the developer. The user controls the interface and its style.

That said we still need content and to be somewhat pragmatic. So we are starting with a subset of html, basically Semantic Html + Markdown types + Form Controls. You may split types up into modules:
- `src/nodes/content.rs`: lists, links, 
- `src/nodes/form.rs`: buttons, checkboxes, etc
- `src/nodes/form.rs`: buttons, checkboxes, etc
- `src/nodes/layout.rs`: not nodes but inserted alongside them, and sometimes required, ie DisplayBlock, DisplayInline

the `Text` parent should be removed, replace this instead with a `Node`. ie a changed `TextNode` will also notify its parent `Node` of the change.

The actual structure of a tree hierarchy more or less mirrors that of html, for instance an ElementNode may have child TextNodes and child ElementNodes.

### Invariance

Nodes are not allowed to change types. they must be despawned, so we need a way to check for that.

```rust
// src/nodes/node.rs
// 
#[derive(Component)]
#[on_add=ensure_exclusive]
pub struct Node{
	/// the type of the node component
	type_id: std::any::TypeId
}

impl Node{
	pub(crate) fn new::<T>()->Self
}
/// 
fn ensure_exclusive(mut world:DeferredWorld, cx: HookContext){
	let node = world.entity(cx.entity).get::<Node>().unwrap();
	todo!("ensure it didnt already have a node of a differnt kind");
	
}
..

#[require(Node = Node::new::<TextNode>())]
pub struct TextNode;
```

## Renderers

Located at `src/renderers`

For a given `Card`, these will produce some output, ie markdown.

These may be triggered by tools, ie the markdown renderer, or by systems, ie the tui `draw_system.rs`.

- CellAlignment seems like a mistake, we should be using the types in `nodes/layout.rs`
- seperate out render_markdown::render() into a type MarkdownRenderer{card_root:bool// default=true}
- self.text_query.is_structural should be enough to check if something is structural

RenderMarkdown is out of control, we need to fomalize a pattern across markdown and tui rendering.
I recommend a visitor pattern but you may find some other fits better, ie a fold pattern with utiltity types to maintain consistency amongst renderers.

```rust
// src/renderers/card_walker.rs

#[derive(SystemParam)]
pub struct CardWalker{
	card_query: CardQuery,
	// used to check what node the entity contains.
	entity_query: Query<EntityRef>
}

impl CardWalker{
	/// Depth-first traversal of a card
	pub fn walk_card(&mut Self, visitor: V, entity: Entity){
		
	}
}
/// Visit each descendant of a card, exclusive of child cards
/// Optionally breaking to stop traversing into this entities children
pub trait CardVisitor{	
	fn visit_entity(&mut self, entity: Entity)-> ControlFlow<(),()>{
		ControlFlow::Continue(())
	}
	fn visit_text(&mut self, node: &TextNode)-> ControlFlow<(),()>{		
		ControlFlow::Continue(())
	}
	etc..
}


```


## Parsers

Located at `src/parsers`

For a given `Card`, these will run a diff with a given `EntityWorldMut` to ensure the tree matches what the parser expects.

```rust
// src/parsers/parser.rs
pub trait Parser{
	fn diff(&mut self, entity: &mut EntityWorldMut)->Result;	
}
```

In general parsers should not need much third party machinery. I'm quite certain that `MdNode` is an entirely useless indirection, we should be able to just diff the string directly.

```rust

pub struct MarkdownDiffer<'a>{
	text: &'a str,
}
impl Parser for MarkdownDiffer {
	..
}
```

Also refactor `demo_stack` to `petes_beets`, a simple stack for a music record store.
