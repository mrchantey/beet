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

I think you got a bit confused here, we should call `visit/leave_entity` on **every** entity, and `visit/leave_unknown_entity` only for the ones we dont know the type of. we're currently calling visit/leave_unknown for known entities!


## Input system

Once thats done lets keep iterating on the generic input system. `tui_mouse_events` has been implemented incorrectly, this is supposed to be renderer agnostic, migrate these to `src/input/mouse_events.rs`.

Then add an observer to the `text.rs::Link`, so when it receives a mouseup will log: 'do something cool here', ill implement that later.


```
#[component(on_add=on_add)]
struct Link

fn on_add(mut world,cx){
	world.commands().entity(cx.entity).add_observer(on_click_link);
}
fn on_click_link(ev:On<MouseUp>)..

```