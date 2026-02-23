- events in pointer.rs should be a regular EntityEvent, not an EntityTargetEvent:

```rust
#[derive(EntityEvent)]
struct Pointer{
	#[event_target]
	target: Entity,
	pointer: Entity
}

impl Pointer{



	pub fn new(pointer: Entity)-> impl FnOnce()->Self{
		|target|{
			Self{
			pointer,
			target
			}
		}
	}
}

commands.entity(foo).trigger(Pointer::new(pointer_entity))

```
