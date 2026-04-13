# beet_action

Functions as entities.

```rust

// pure actions dont interact with the world
#[action(pure)]
async fn Add(cx: ActionContext<(i32,i32)>) -> i32 {
	cx.0 + cx.1
}

let out = World::new().spawn(Add)
	.call::<(i32, i32), i32>((1, 2))
	.await
	.unwrap();

assert_eq!(out, 3);
```
