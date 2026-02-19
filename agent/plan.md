lets keep iterating on `crates/beet_tool`

- system_tool: it needs SystemToolIn, just like FuncToolIn and AsyncToolIn, the input param for the system would then be `In<SystemToolIn<Foo>>`
- wrap_tool: `IntoWrapTool` needs to be implemented for all IntoToolHandler, where the In is a tuple of (Foo, Next). replace the `WrapFn` pattern with a blanket impl