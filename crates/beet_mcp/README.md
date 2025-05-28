# `beet_mcp`

An mcp server for rust developers, currently only exposing a single tool: `crate_rag`.

## Quickstart

Models can be run locally or in the cloud, give local a go if you have an NVIDIA or AMD GPU with [at least 5GB](https://claude.ai/share/f375b98b-820d-4c5d-bb52-9f731353e976) of RAM, anything from the last 5 years should be fine.

### Quickstart - Local (recommended)

Running models locally lets us use the mcp without incurring any costs.

1. Install [`ollama`](https://ollama.com/download) and some models
	```sh
	# install ollama
	curl -fsSL https://ollama.com/install.sh | sh
	# a tiny 45MB embedding model used by `cargo test` 
	ollama pull all-minilm:latest
	# a medium 700MB embedding model used by the mcp in `cargo run`
	ollama pull mxbai-embed-large:latest
	# a large 5GB completion model used by `cargo run --example agent`
	ollama pull qwen3:latest
	```
2. Index the databases and run the example agent
	```sh
	cargo run --bin index-all
	cargo run --example agent
	```

### Quickstart - Open AI

This is the fastest way to get started and good option if you don't have a decent GPU, but indexing does incur a small fee (by my estimates indexing the bevy repo would cost about $0.30)

1. Create a `.env` with your openai key:
	```sh
	OPENAI_API_KEY=https://platform.openai.com/api-keys
	```
2. index the databases and run the example agent
	```sh
	cargo run --bin index-all --features=openai
	cargo run --example agent --features=openai
	```

### Other Models

If you would like to experiment with other models, for example if openai releases a new model, or if you have a small or large GPU, you may want to experiment with a different model.

For example I have a 3080(12GB) so wanted to try [a bigger qwen3 model](https://ollama.com/library/qwen3).
```sh
# trying a different ollama model
ollama pull qwen3:14b
# .env
BEET_MODEL_AGENT_OLLAMA=qwen3:14b
# or for trying a new openai model
BEET_MODEL_AGENT_OPENAI=GPT_4_5_PREVIEW_2025_02_27
```
See [`.env.example`](.env.example) for all options.


### Running MCP Servers

Agents primarily communicate with MPC servers in one of two ways:
- via stdio, ie the agent will call the executable, (or `cargo run` during development).
- via http Server Side Events (sse)

See the commented out `sse` parts of [`./examples/mcp_server.rs`](./examples/mcp_server.rs) and [`./examples/mcp_client.rs`](./examples/mcp_client.rs) for details.

### Discovering MCP Servers

During development its usually easier to work with an agent 'in code', see []

For 'out in the wild' agents like cursor, claude code, vscode etc, an `mcp.json` is used. See [`.vscode/mcp.json`](.vscode/mcp.json) for an example of calling this server with the vs code agent during development.


## `crate_rag`

Vector Databases have two phases:
1. create embeddings for the content: `cargo run --bin index-all`
2. run a query against the database: `cargo run --example repo_query`

### How it works

This tool is essentially glue code for several crates:

- `rig-core`: agentic ai crate used for running models and working with vector databases
- `rmcp`: the official rust mcp sdk
- `rustdoc-md`: generate markdown documents from `cargo doc`
- `text-splitter`: split a document, ie `.md`, `.rs` into chunks

The only part I'd consider specialized is the [`ContentType`](src/crate_rag/content_type.rs) layer, which allows us to split `docs`, `examples`, `src`, `guides`.

### MCP testing

The mcp inspector can be used for visual testing:
```sh
npm i -g @modelcontextprotocol/inspector
npx @modelcontextprotocol/inspector cargo run
```

### Findings

#### Source code vs public apis

Indexing examples and source code together can *worsen* the agent's performance.
For example `create a 2d camera` will likely return the definition of `Camera2d`, encouraging the agent to *create a 2d camera from scratch* just like the source code.

### Future Work

- `bevy_remote` mcp
- a more sophisticated approach than `ContentType`, perhaps something like [`graphrag`](https://microsoft.github.io/graphrag/)