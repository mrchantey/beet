lets keep iterating on beet_stack!

# `style.rs`

`InlineStyle` check your working on `	pub link: Option<Link>`, im quite sure that should just be another bitflag instead of cloning the node, if its even nessecary? Like why is the tui renderer using this instead of visit_link. if hyperlink impl is wrong fix that instead of this hack.

# `card_walker.rs`

This needs a bit of a refactor, this `DispatchKind` thing is very weird. I'd prefer to not be passing around the instruction to the function and then doing a match on it. Try something less clever, i have no idea if this will work but something like this you wont need to do all that weird passing around.

```
fn per_entity(){
 call visit entity
 match type, call visit inner
 iterate on children
 match type, call leave inner
}
```

- In the visitor trait move the entity to VisitContext, making the api cleaner with only one param.
- Replace `ctx` var name with `cx`

# `markdown_macro.rs`

This does not need World, instead do the same architecture as `content_macro.rs` with a trait `IntoBundle<M>`

```rust
impl IntoBundle<Self> for &str {
	fn into_bundle(self) -> impl Bundle { 
		OnSpawn::new(self, |entity:EntityWorldMut|{
			// do the markdown diffing, inserting etc here.
		})		
 }
}
let bundle = markdown!(r#"

# My Site

Welcome to my site	
"#
{(Paragraph::with_text("interspersed with bundles"))}
"And some *more text after*"
)
```
I'm guessing this means the top level is a flat list of children, so the above example is an empty entity with three children.

When done remove the `content_macro` completely and replace all usage with this one. Also check all spawning of TextNode::new or xx::with_text, in almost all cases prefer usage of markdown! for easier readability, unless testing the macro itself or some nuance case.

## `render_tui.rs`

In general the formatting should be more in line with markdown, maybe two preceeding newlines for the main heading, newlines around block quotes etc. expose sensible config options on the renderer.
Also verify that links and buttons are rendering correctly, see `hyperlink` and `button` widget for the tui stuff. For me its not clickable, and seems to be Block instead of Inline.

## Testing

- The markdown! macro enables more ergonomic testing of our renderers and parsers, For more advanced 'kitchen sink' rendering and parsing tests, use `my_str.xpect_snapshot()`
