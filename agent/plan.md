
lets keep iterating on beet_stack!


## Non-Async Tool usage

Non-async tool usage should not require AsyncCommands. 

ToolCall should be generic on its SystemParam, not hardcoding AsyncCommands.
- grep all usage of `AsyncPlugin::world`


The last iteration was pretty good but we ended up with a bit of confusion so there are some mistakes in the codebase.

## InsertRouteTree

insert_route_tree needs some work. its entirely overengineered. we should use exactly the same mechanism as a formal request, the `CardContentFn` is an antipattern. This means that inserting the route tree will be asynchronous as it will need to individually and recursively call each route entity. Also remove `CardContentHandler`.


## `card.rs`
the card() must accept a regular `IntoToolHandler` which resolves to a bundle, not this bespoke Fn(). Because these are typed we must use an intermediary spawn tool that will get the typed bundle, insert it, and return the entity that was spawned.

```rust
fn card<Handler, Out:B>(..,handler:Handler)->impl Bundle where Handler: IntoHandler<In=Request,Out=Out>{

	OnSpawn::new(move |entity|{
		let typed_card = entity.world_scope(|world|world.spawn(handler.into_card_handler()).id());
		let spawn_card = entity.world_scope(|world|world.spawn(spawn_card).id());
		
		entity.insert(..the tool that receives a request, uses the spawn_card id as the RenderRequest::handler, to spawn the entity)
	})
}
	
// a tool with input Request and output Entity
fn spawn_card<B:Bundle>(typed_card:Entity)->impl Bundle{
	tool(|req:Request|{
		.. call the typed_card with the request, receiving B
		.. spawn the returned bundle and return the spawned entity
	})
}
```



## Testing

aside from `cargo test -p beet_stack`, also run 

`cargo run --example tui --features=tui` with timeout cos if it succeeds will not return. ensure this is rendering correctly.
also run variants passing in the initial commands
`cargo run --example repl --features=stack`
`cargo run --example repl --features=stack -- --help`
`cargo run --example repl --features=stack -- counter --help`
`cargo run --example repl --features=stack -- counter increment`

`cargo run --example tui --features=tui`
`cargo run --example tui --features=tui -- --help`
`cargo run --example tui --features=tui -- counter --help`
`cargo run --example tui --features=tui -- counter increment`
