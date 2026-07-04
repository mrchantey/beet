# Perceive-act

A small floor robot living a perceive-act loop: it takes a photo, describes what it sees, picks an emotion, says a line in character, then chooses a heading, over and over. The loop runs until you stop it with Ctrl+C.

The agent is a socket server (`agent.bsx`, shared by every version); only the head and body clients change between versions. Run every command from the repo root.

## Prerequisites

- `OPENAI_API_KEY` in `.env` (loaded automatically), used by the agent and the vision describe.
- Assets: `just pull-assets`, for the floor-photo fixtures, the fox model, and the robot-eyes face sprites.
- Optional: the kokoro `tts` command on `PATH` for spoken audio. Without it, speech is logged and skipped.

## v1: mock head and body

One process, both clients mocked: the head reads the floor photos and logs the emotion, the body logs the heading.

```sh
cargo run -p beet-cli --features sockets -- --main=examples/perceive_act/main-v1.bsx --root=. --server=socket
```

## v2: wgpu fox body

Same mock head, but the body is a 3d fox in a window that drives off each heading.

```sh
cargo run -p beet-cli --features winit,sockets -- --main=examples/perceive_act/main-v2.bsx --root=. --server=socket
```

## v3: browser head

No in-process clients. A second HTTP server serves a wasm browser head that connects back over the socket, serving the real webcam, Web Speech, and a rendered face. Build the head wasm once, then run both servers:

```sh
beet build-wasm --package=beet-cli --bin=beet --features=web_head --out=examples/perceive_act/head/assets/perceive-act-head.wasm
cargo run -p beet-cli --features thread,sockets -- --main=examples/perceive_act/main-v3.bsx --root=. --server=socket,http
```

Then open http://127.0.0.1:8337 and grant camera access. The `/debug` route shows the webcam, face, and log on one page.

### Real device body (optional)

Instead of the fox, an Arduino Alvik can connect as the body and drive off each heading. From the adjacent `../beet_esp`, with the device plugged in (see that repo for toolchain and flashing details):

```sh
cargo run --release --features alvik,sockets        # flash the firmware + monitor
beet load templates/alvik/perceive-act-body.bsx     # push the body scene (BEET_REMOTE_URL -> device)
```

Set the `url` in `perceive-act-body.bsx` to this host's socket server (`ws://<host>:8338`).
