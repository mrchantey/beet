not a bad iteration on beet_stack chicken, lets keep iterating.

- `TuiWidget` should store something more generic, use one of: `ratatui::Widget`, `ratatui::WidgetRef`, `ratatui::StatefulWidget` or `ratatui::StatefulWidgetRef`.
- refactor Title and TitleLevel, use Heading1(require(Heading = Heading::new_level_one)), Heading2(require(Heading = Heading::new_level_two)) etc. to be clear, this means no more `calculate_title_level` etc
```rust
/// Cannot be constructed except by a Heading1 etc.
struct Heading{
	level: u8
}


impl Heading{
	pub fn level(&self)->u8..
	fn new_level1()->Self...
}
```
- Refactor this concept of 'Heading/Paragraph' being structural elements. instead introduce DisplayBlock, where Heading and Paragraph both require `DisplayBlock`. put those in `layout.rs` and replace all the Title or Paragraph checks with this DisplayBlock check.
- move HyperLink and Button into `tui_server/widgets` and remove the `Tui` prefix, thats very non-rust
- ToolMeta::of shoud accept an `IntoToolHandler`, storing the std::any::type_name of `H` as well as `ToolMeta::name`. then use that as the button names in draw_system
- what the fuck is `extract_paragraph_text`, get rid of this. if its a `Heading` then it has child TextContent, see `TextQuery::collect_text` 
- all this `		// Calculate content height` stuff for the scrollbar looks unnescecary, look closely at `agents/ratatui/examples/apps/scrollbar/src/main.rs`, i dont see them doing all that weirdness it should be simple and mostly abstracted into `widgets/scrollbar.rs`
- regarding scrollbar the page is not rendering correctly for a variety of reasons, double check the scrollbar logic is consistent with the demo.
- add the TextQuery::main_heading helper method, abstracting out of the draw_system usage
- in general the draw system should not be so imperative, lean into structure and widgets, then compose together. see `agents/ratatui/examples/apps/widget-ref-container/src/main.rs` for an example of a generic container
