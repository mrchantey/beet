# beet_tool

Functions as entities.

```rust

#[tool]
fn add(a:i32, b:i32) -> i32 {
	a + b
}

let out = World::new().spawn(add.into_tool())
	.call::<(i32, i32), i32>((1, 2))
	.await
	.unwrap();

assert_eq!(out, 3);
```
