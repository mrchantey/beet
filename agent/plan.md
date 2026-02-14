beet_stack iterations are coming along nicely chicken.

- refactor `collect_text_entities` to actually return a Vec<(Entity,&TextContent)>, thats more useful.





- all this `		// Calculate content height` stuff for the scrollbar looks unnescecary, look closely at `agents/ratatui/examples/apps/scrollbar/src/main.rs`, i dont see them doing all that weirdness it should be simple and mostly abstracted into `widgets/scrollbar.rs`
- regarding scrollbar the page is not rendering correctly for a variety of reasons, double check the scrollbar logic is consistent with the demo.

- in general the draw system should not be so imperative, lean into structure and widgets, then compose together. see `agents/ratatui/examples/apps/widget-ref-container/src/main.rs` for an example of a generic container
