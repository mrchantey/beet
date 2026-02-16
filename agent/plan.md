lets keep iterating on beet_stack rendering!

`text.rs` refactor Important, Bold etc into actual Inline Nodes just like in html. This means TextQuery will need to be updated to support this, or removed completely? its currently broken anyway and was introduced as a bit of a lazy hack.
I think we need to lean into a more consistent rendering pattern. Now we have `card_walker.rs` lets migrate both the markdown renderer and tui renderer:

## Markdown

```rust
// src/renderers/markdown_renderer.rs
pub struct MarkdownRenderer<'a>{
	buffer: &mut str,
}

impl Visitor for MarkdownRenderer<'_>{
	..
}
```

## Tui

```rust
// src/renderers/tui_renderer.rs

pub struct TuiRenderer<'a>{
	// perhaps area gets pushed and popped? i dont know.
	// We also need to consider how scrolling will work, generally we need to calculate the entire height of the content.
	// we can also add after_visit_entity() after_visit_entity_children etc to the visitor if that helps.
	area: &mut Rect,
	buffer: &mut Buffer
}

```

Note that the `crossterm` tui backend is difficult to test. for example the rendering is currently completely broken but i have no way to show that to you.

Lets create a mock backend that we can set the `Rect` for, with an easily accesible buffer. Then we can write tests to verify that the content and ansii codes etc are actually coming out as expected, ie nesed Bold text and all that. perhaps we can verify against the `ratatui::Buffer` and `Cell` types instead of dealing with opaque ansii characters.
not sure if this means wrapping `RatatuiContext` in some kind of `TuiContext` enum.
we'll need lots of assertions so maybe even create a seperate cfg test module `mock_tui_backend` or something with lots of helper methods.

Also verify that links and buttons are rendering correctly, see `hyperlink` and `button` widget.
