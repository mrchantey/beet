
### Shared Actions

Sometimes we need to communicate with a parent action, this can be done by marking an action field as `#[shared]`.

```rust
pub struct SomeScoreSetter{
	#[shared]
	pub score: Score
}
```

### Selector Actions




### Trees

Trees are a collection of actions and other trees. To reduce boilerplate they can be defined with [rsx](https://crates.io/crates/rstml).
```rs
#[tree_builder]
pub fn MyTree() -> impl TreeElement {
	tree! {
		<sequence>
			<say_hello/>
			<SayWorld/> //another tree declared elsewhere
		</sequence>
	}
}
```

> The `tree!` macro uses the web UI naming convention:
> - `actions` have snake_case
> - `trees` have PascalCase

## Running

- A `TreePlugin` schedules all systems in the tree:  
	```rust
	app.add_plugins(TreePlugin::new(MyTree));
	```
- A `TreeBundle` adds props to specified nodes in the tree:
	```rust
	app.world.spawn(TreeBundle::root(MyTree, Running));
	```

Putting it all together:

```rs
fn main(){
	let mut app = App::new();	
	app.add_plugins(TreePlugin::new(MyTree));
	app.world.spawn(TreeBundle::root(MyTree, Running));

	app.update(); // runs first child
	app.update(); // runs second child
}
```
```sh
> cargo run
hello
world
```
<!-- > This example uses `bevy`, see [no_bevy](./no_bevy) for more examples. -->

[1]: https://crates.io/crates/bevy_ecs
