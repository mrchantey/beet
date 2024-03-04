# Action Timers


Action start/stop times are commonly used for task-switching, and included in `beet`.

Simply add an `ActionTimer` to entities you want to track that with, and they will be updated automatically.

## Example

```rust
...
	app.world.spawn(TreeBundle::recursive(my_tree, ActionTimer::default()));
...

#[action]
pub fn succeed_in_one_second<N: AiNode>(
	mut commands: Commands,
	mut query: Query<(Entity, &Prop<ActionTimer, N>), With<Prop<Running, N>>>,
) {
	for (entity, timer) in query.iter_mut() {
		if timer.last_start.elapsed() >= Duration::from_secs(1) {
			commands
				.entity(entity)
				.insert(Prop::<_, N>::new(ActionResult::Success));
		}
	}
}
```