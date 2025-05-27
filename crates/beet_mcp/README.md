# `beet_mcp`

An mcp server.

## Crate Rag

### MCP testing

The mcp inspector can be used for visual testing:
```sh
npm i -g @modelcontextprotocol/inspector
npx @modelcontextprotocol/inspector cargo run
```




### Findings

#### Source code vs examples

Indexing examples and source code together can *worsen* the rag.
For example `create a 2d camera` will likely return the definition of `Camera2d`, encouraging the llm to *create a 2d camera from scratch* just like the source code.


### Future Work

- `bevy_remote` mcp
- 