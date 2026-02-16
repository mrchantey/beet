lets keep iterating on beet_stack!

# `style.rs`

`InlineStyle` check your working on `	pub link: Option<Link>`, im quite sure that should just be another bitflag instead of cloning the node.

# `card_walker.rs`

This needs a bit of a refactor, this `DispatchKind` thing is very weird. It seems we could just do a hamburger thing:

```

```

- Ditch , we should be able to just pass the NodeKind. if need be create a seperate method for leaving an entity.
- Move the entity to VisitContext, making the api cleaner.
- Replace `ctx` var name with `cx`

# `markdown_macro.rs`

This does not need World, instead do the same architecture as `content_macro.rs` with a trait `IntoBundle<M>`

```rust
impl IntoBundle<Self> for &str {
	fn into_bundle(self) -> impl Bundle { 
		OnSpawn::new(self, |entity:EntityWorldMut|{
			// do the markdown diffing etc here.
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

I'm not sure what this means for hierarchies you'll need to work that out.

When done remove the `content_macro` completely and replace all usage with this one. Also check all spawning of TextNode::new or xx::with_text, in almost all cases prefer usage of markdown! for easier readability, unless testing the macro itself or some nuance case.

## `render_tui.rs`

In general the formatting should be more in line with

## Testing

- The markdown! macro enables more ergonomic testing of our renderers and parsers, For more advanced 'kitchen sink' rendering and parsing tests, use `my_str.xpect_snapshot()`
- Also verify that links and buttons are rendering correctly, see `hyperlink` and `button` widget for the tui stuff.