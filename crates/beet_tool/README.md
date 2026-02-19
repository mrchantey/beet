# beet_tool

Entities as async functions.


```rust

#[tool]
fn add(a:i32, b:i32) -> i32 {
	a + b
}

#[allow(non_snake_case)]
#[derive(BundleEffect)]
struct add;

impl add{	
	fn effect(entity: &mut EntityWorldMut){
		entity.insert(ToolHandler::new(..))
	}
}

world.spawn(add)
	.call_blocking::<(i32, i32), i32>((5, 5))
;
```