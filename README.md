# beet

<div align="center">
  <p>
    <strong>A creative tool engine</strong>
  </p>
  <p>
    <a href="https://crates.io/crates/beet"><img src="https://img.shields.io/crates/v/beet.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/beet"><img src="https://img.shields.io/crates/d/beet.svg?style=flat-square" alt="Download" /></a>
    <a href="https://docs.rs/beet"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
  </p>
  <h3>
     <a href="https://beet.org">Website</a>
     <span> | </span>
    <a href="https://docs.rs/beet">API Docs</a>
  </h3>
</div>

Beet helps you build the perfect stack for cross-domain projects. Websites, agents, robots, games and infra are all under one roof with great defaults and deep extensibility.

Beet is built on the [Bevy Engine](https://bevy.org) and its Entity Component System architecture. See [the beet website](https://beet.org/docs) for more info.

> 🚧 Mind your step! 🚧
>
> Beet is under construction, if this project is of interest please come and say hi in the [Beetmash Discord Server](https://discord.gg/DcURUQCXtx).

## Example - Embodied Agents

Beet is a natural fit for distributed systems like embodied agents with a perceive-act loop. A perceive-act agent is made of three apps: a server for the resources, a smartphone for the head and an ESP32 for the body. The server is an agent whose routes are the capabilities, forwarding over the socket to whichever socket client serves them.

```jsx
<Router {(SocketServer, BootOnLoad, CapabilityServer)}>
	<!-- routable by interpret-photo, not offered to the model -->
	<TakePhoto/>
	<div {RepeatWhileFunctionCallOutput} {CreateThread}>
		<div {Thread} {Sequence}>
			<CreateActor name="System" kind="System">
				<CreatePost text='
You are a small, curious and very emotional floor robot exploring a room.
You perceive the world one photo at a time and act on what you see.
...
'/>
			</CreateActor>
			<CreateActor name="Robot" kind="Agent" {ModelStreamer{provider:OpenAi}}>
				<InterpretPhoto/>
				<SpeakText/>
				<SetEmotion/>
				<ApplyHeading/>
			</CreateActor>
		</div>
	</div>
</Router>
```

The head is a web page whose tab serves `take-photo` (webcam), `speak-text` (speech synthesis) and `set-emotion` (the face on screen). The body is an ESP32 serving `apply-heading`, steering in the chosen direction. The full example, including mocked and 3d-rendered stages for running without hardware, lives at [examples/perceive_act](examples/perceive_act).

## Crates

The beet project closely resembles the bevy project with modules divided into crates which can be selected via feature flags.

**readiness meter**
- 🦢 ready to go: documented and tested
- 🐣 near stable: incomplete docs
- 🐉 highly experimental: here be dragons

The `beet` crate re-exports the crates below behind feature flags. Each can also be used standalone.

### Core

Cross-platform primitives shared by every other crate.

| Crate                                          | Status | Description                                          |
| ---------------------------------------------- | ------ | ---------------------------------------------------- |
| [`beet_core`](crates/beet_core)                | 🦢      | Cross-platform types, extension traits and testing |
| [`beet_net`](crates/beet_net)                  | 🐣      | Transport agnostic networking      |
| [`beet_action`](crates/beet_action)            | 🐣      | Entities as functions                 |
| [`beet_ui`](crates/beet_ui)                    | 🐉      | Interface agnostic XML-like UI trees   |
| [`beet_router`](crates/beet_router)            | 🐉      | Transport agnostic routing     |
| [`beet_infra`](crates/beet_infra)              | 🐉      | Infrastructure as code, built on OpenTofu            |
| [`beet_async`](crates/beet_async)              | 🐉      | Vendored bevy_async bridge for wasm and exclusive world access |

### Agents & Behavior

Behaviors built on `beet_action`, for paradigms like behavior trees, utility AI and agentic systems.

```rust
use beet::prelude::*;

# async fn run() -> Result {
let outcome = AsyncPlugin::world()
  .spawn((Sequence::new(), children![
    Log::new("hello"),
    Log::new("world"),
  ]))
  .call::<(), Outcome>(())
  .await?;
# Ok(()) }
```

| Crate                                            | Status | Description                                          |
| ------------------------------------------------ | ------ | ---------------------------------------------------- |
| [`beet_thread`](crates/beet_thread)              | 🐉      | Multi-actor chat for humans and agents |
| [`beet_spatial`](crates/beet_spatial)            | 🐉      | Spatial actions: movement, steering and robotics     |
| [`beet_ml`](crates/beet_ml)                      | 🐉      | Machine learning actions: embeddings and RL          |

### Apps & Tooling

| Crate                                            | Status | Description                                          |
| ------------------------------------------------ | ------ | ---------------------------------------------------- |
| [`beet-cli`](crates/beet-cli)                    | 🐉      | The primary entrypoint for a vanilla beet app      |
| [`beet_extra`](crates/beet_extra)                | 🐉      | Extra components and systems for high-level examples |

## Bevy Versions

| `bevy` | `beet`  |
| ------ | ------- |
| 0.19   | 0.0.9   |
| 0.17   | 0.0.7   |
| 0.16   | 0.0.6   |
| 0.15   | 0.0.4   |
| 0.14   | 0.0.2   |
| 0.12   | 0.0.1   |

## Local Development

### Running

Note that testing all involves compiling *many* crates, doing so from scratch usually results in a stack overflow in the rust compiler.
To prevent this either run with RUST_MIN_STACK='some_gigantic_number', or just keep re-running the command until its all compiled.

```sh
git clone https://github.com/mrchantey/beet
cd beet
just init-repo
just test-core
```
