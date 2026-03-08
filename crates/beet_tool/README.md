# beet_tool

Entities as async functions.


```rust

#[tool]
fn add(a:i32, b:i32) -> i32 {
	a + b
}

world.spawn(add)
	.call::<(i32, i32), i32>((5, 5))
	.await
;
```
