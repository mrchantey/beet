
lets keep iterating on beet_stack!

## Tool Refactor Complete

This just in! We have two new major features to tool which will radically simplify this crate.

1. `pipe_tool.rs` for tools that need to chain into other tools, very useful, for example piping a card spawn tool to a a render tool. 
2. `wrap_tool.rs` for wrapping a tool inside another aka middleware. this will make our Request/Response wrappers of internal tools much simpler!

In general we've been hot-potatoing, creating bespoke ToolHandler::new calls, spawning extra nested tool entities etc. this should all be much simpler now.

## InsertRouteTree

insert_route_tree needs some work. its entirely overengineered. we should use exactly the same mechanism as an actual formal request, the `CardContentFn` is an antipattern. This means that inserting the route tree will be asynchronous as it will need to individually and recursively call each route entity. Also remove `CardContentHandler`.

## `card.rs`
the card() must accept a regular `IntoToolHandler` which resolves to a bundle, not this bespoke Fn(). Use wrapping and piping as required.


Give the crate a once-over, shave off these rough edges and simplify the design with these new primitives.


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
