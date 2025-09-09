# `beet_mcp`

Experimental mcp server, currently only exposing a single tool: [`crate_rag`](#crate_rag).

## Quickstart

There's a few moving parts in this crate, and it currently requires `nightly`. Its also currently separate from the rest of the repo so better to cd directly into it.

```sh
git clone https://github.com/mrchantey/beet
cd beet/crates/beet_mcp
```

`sqlite` is required for the vector databases.
```sh
sudo apt-get install sqlite3
```

Models can be run locally or in the cloud, I'd recommend giving local a go if you have an NVIDIA or AMD GPU with [at least 5GB](https://claude.ai/share/f375b98b-820d-4c5d-bb52-9f731353e976) of RAM, anything from the last 5 years should be fine.

### Quickstart - Local (recommended)

1. Install [`ollama`](https://ollama.com/download) and these three models used for tests and examples.
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

You may want to experiment with a different model if a new one comes out, or if you have a small or large GPU. For example I have a 3080(12GB) so wanted to try [a bigger qwen3 model](https://ollama.com/library/qwen3).
```sh
# trying a different ollama model
ollama pull qwen3:14b
# .env
BEET_MODEL_AGENT_OLLAMA=qwen3:14b
# or for trying a new openai model
BEET_MODEL_AGENT_OPENAI=GPT_4_5_PREVIEW_2025_02_27
```

fyi for this mcp it didn't make much of a difference. See [`.env.example`](.env.example) for all options.


### Running MCP Servers

Agents primarily communicate with MPC servers in one of two ways:
- via stdio, ie the agent will call the executable, (or `cargo run` during development).
- via http Server Side Events (sse)

See the commented out `sse` parts of [examples/mcp_server](./examples/mcp_server.rs) and [examples/mcp_client](./examples/mcp_client.rs) for details.

### Discovering MCP Servers

During development its usually easier to work with an agent as rust code, see [examples/agent.rs](./examples/agent.rs).

For 'out in the wild' agents like cursor, claude code, vscode etc, an `mcp.json` is used. See [.vscode/mcp.json](.vscode/mcp.json). Agents like to query from all kinds of directories so i haven't yet worked out where to put the cache for stdio so recommend the sse approach.

```sh
cargo run --bin sse-server
```
option 1: use the inspector
```sh
npx @modelcontextprotocol/inspector
```
option 2: add this to mcp.json
```json
{
	"servers": {
		"beet-mcp": {
			"type": "sse",
      "url": "http://127.0.0.1:8000/sse",
		}
	}
}
```


## `crate_rag`

The goal of this tool is, for a given [`CrateRagQuery`](src/mcp/mcp_server.rs#L25), to return the top n chunks of data that match it. An example query might look like this:

```json
{
	"crate_name": "bevy",
	"crate_version": "0.16.0",
	"content_type": "examples",
	"max_docs": 4,
	"search_query": "3d camera controller"
}
```

For this to work we need to know the git url and commit hash for that version, finding a nice way to handle that is a work in progress but its currently hardcoded, see [`KnownSources`](src/crate_rag/known_sources.rs#L166-L167). Feel free to add as many as you like,

 By default the only sources indexed by [index-all](src/bin/index-all.rs) are `bevy 0.16` docs, examples and guides. eventually these should be cli args.


`beet_mcp` uses the approach of a [Vector Database](https://www.cloudflare.com/learning/ai/what-is-vector-database/#:~:text=A%20vector%20database%20stores%20pieces,construction%20of%20powerful%20AI%20models.) to store and query the repository.
This differs from grepping techniques in that it can match phrases that are similar in *meaning* instead of exact terms, the tradeoff being that we need to index the db beforehand.

Vector DB rag has two phases:
1. create embeddings for the content: `cargo run --bin index-all`
2. run a query against the database: `cargo run --example repo_query`

Under the hood the tool is using several crates:

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

### Contributing

Running `cargo test` requires the steps in the local quickstart, except only the `all-minilm:latest` model is required. You will notice references to the sample document [nexus_arcana](nexus_arcana.md) used for unit tests.


### Future Work

- `bevy_remote` mcp
- a more sophisticated approach than `ContentType`, perhaps something like [`graphrag`](https://microsoft.github.io/graphrag/)
