# beet_action

Functions as entities.

- no_std
- async

```rust

// pure actions, very fast.
#[action(pure)]
fn Add(cx: ActionContext<(i32,i32)>) -> i32 {
	cx.0 + cx.1
}

// system actions, world access
#[action(pure)]
async fn OnlineUsers(_cx: ActionContext, users: Query<&User>) -> usize {
	users.iter().count()
}

// async actions, async world access
#[action(pure)]
async fn GetWeather(cx: ActionContext<String>) -> String {
	let weather = cx.caller.clone_resource::<HttpClient>()
		.get(format!("weather.com/&location={}", cx.value()))
		.send()
		.await
		.text()
		.await?;
	Ok(format!("The weather today is {weather}"))
}


// Actions are then inserted as components 
// and called by Commands or EntityWorldMut
let out = World::new()
	.spawn(Add)
	.call::<(i32, i32), i32>((1, 2))
	.await
	.unwrap();

assert_eq!(out, 3);
```
