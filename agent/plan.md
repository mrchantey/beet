lets keep iterating on our beet_stack tui renderer


- signatures that accept two params like this are unacceptable, its too easy to conflate row/col. use TuiPos
	- `TuiSpanMap::get(&self, col: u16, row: u16) -> Option<Entity>`

## card_walker.rs

```
# previous instructions
-rename visit_entity to visit_unknown_entity.
- create a new visit_entity that is called before visiting any of the typed versions
- also create leave_entity and leave_unknown_entity
```

I think you got a bit confused here, we're calling `visit/leave_unknown_entity` for known entities! this is only for the ones we dont know the type of. we call `visit/leave_entity` on **every** entity

## tui_renderer.rs

- `renderer.finish()` should take self, not &mut self.


## tui_span_map.rs
