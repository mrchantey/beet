# THE GREAT DISCOVERY

`draw_system.rs` we're currently trying to cache created Widgets as entities in the tree.
this will never work because widgets are themselves trees, ie the ratatui::Paragraph,
so we end up with this weird duplicate tree situation.

## THE GREAT SOLUTION

lets give up on this caching. now we'll create a 