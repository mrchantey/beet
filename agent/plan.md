## card_walker.rs

-rename visit_entity to visit_unknown_entity.
- create a new visit_entity that is called before visiting any of the typed versions
- also create leave_entity and leave_unknown_entity

## tui_renderer.rs

- Entity::PLACEHOLDER is an antipattern, use Option<Entity>
- setting self.current_entity should be on visit entity, likewise setting to None on leave entity
- remove `take_span_map`, return the span map on `renderer.finish(self)`

## tui_span_map.rs

do not use a tuple, use a struct:

```rust
struct TuiPos{
	row: u16,
	col: u16,
}
impl TuiPos{
	pub fn new(row:u16, col: u16)
}
```

## tui_mouse_events.rs

do not use a tuple, use a named field:

```rust
pub struct TuiMouseDown{
	target: Entity
};
```


## Behavior

It seeems that clicking on a text span always just resolves to the line, for instance
`this is some **Bold Text**`
where Bold Text is actually a different entity, it should resolve at the span level, not the entire line.
