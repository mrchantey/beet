
lets keep iterating on beet_stack!

IntoToolOutput is absolutely killing us. 

Disambiguating between `T, Result<T>, impl Future<Output=T>, impl Future<Output=Result<T>>` has proved absolutely impossible. its also quite ambiguous. We need to be able to return any type from a tool.

At this point im wondering if we should remove `IntoToolOutput` completely and use explicit funcs for tool definition, just to straighten out the output types. Even doing this im not sure how that would work, currently the fact that Future doesnt impl Typed and is therefore not 

```rust
// returns a tool that returns the output verbatim
// if an async tool is provided, the output is a future.
fn tool(my_tool)
// unwraps the result, see flatten.rs
tool(my_tool.pipe(flatten));
// unwraps the future and 
fn async_tool(my_tool)
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
