## Tool macro

- tool.rs#28 abstract into collect_params func

consider this system tool:
```rust
	#[tool]
	fn sys_with_resource(val: In<i32>) {
		let val:In<i32> = val;
	}
```
this does not compile because we are unwrapping the In, we need to do that to get the `input`, but then we should put that back in an In() so we aren't lying to the user. add a test in system_tool to ensure this.


## Migrate `beet_stack` to use `beet_tool`

completely remove the `src/tools` directory in beet_stack, it should now depend on `beet_tool`. lean into the #[tool] macro for tool definitions.

Also consider that each of these tool will also need a constructor:

```rust
fn fallback()->impl Bundle{ fallback_tool().into_tool_handler()}
#[tool]
fn fallback_tool(..){..}
```


Note that some of the patterns in beet_stack are outdated and there may be opportunities to streamline, using these new `wrap_tool` and `chain_tool` (formerly pipe_tool) patterns.
