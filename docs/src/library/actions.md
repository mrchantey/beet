### Actions

> Currently actions must implement a whole bunch of derives, ie reflect, hence the `#[derive_action]` wrapper for lazy people like me. I'm working on getting this down to as few as possible.

```rust
#[derive_action]
pub struct LogOnRun(pub value: String);

fn log_on_run(query: Query<&PrintAction, Added<Running>){
	for (action) in query.iter(){
		println!("{}", action.0);
	}
}
```

Any action can finish the run state by adding a `RunResult`.

```rust
#[derive_action]
pub struct SucceedOnRun;

fn log_on_run(
	mut commands: Commands, 
	query: Query<Entity, (With<SucceedOnRun>, Added<Running>)
	){
	for entity in query.iter(){
		commands.entity(entity).insert(RunResult::Success);
	}
}
```

Actions can modify their associated agent with the `TargetAgent` component.

```rust
#[derive_action]
pub struct TranslateAgent(pub Vec3);

fn translate_agent(
	mut agents: Query<&mut Transform>,
	query: Query<(&TargetAgent, &TranslateAgent), With<Running>>
	){
	for (target_agent, translation) in query.iter(){
		if let Some(transform) = agents.get(target_agent){
			transform.translation += translation.0;
		}
	}
}
```
