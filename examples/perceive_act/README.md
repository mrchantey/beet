# Perceive-act

A small floor embodied agent living a perceive-act loop: each cycle it is shown a fresh photo and answers with a single `respond-multi-modal` tool call, setting its face, saying one line in character and choosing a heading, over and over. The loop runs until you stop it with Ctrl+C.

One model call per cycle: the `Camera` actor captures via the `take-photo` route and posts the photo straight into the thread, then the `Robot` agent (OpenAI `gpt-5.4-mini`, reasoning off, forced tool choice) answers with one `respond-multi-modal` call, which fans out to `set-emotion`/`speak-text`/`apply-heading` concurrently and awaits them all, so the next photo waits for the body to stop moving. A cycle lands around 2 seconds plus speech.

The agent is a socket server (`agent.bsx`, shared by every version); only the head and body clients change between versions. Run every command from the repo root; the `just` recipes bake in a clean log filter (`RUST_LOG=info,beet_net=warn`), so the per-cycle output reads like:

```
cycle 3: photo captured (100KB in 0.24s, previous cycle 1.76s)
cycle 3: Surprised | "Whoa, hanging basket!" | Right (model 1.66s)
cycle 3: acted in 0.10s (set-emotion 0.07s | speak-text 0.10s | apply-heading 0.07s)
```

## Prerequisites

- `OPENAI_API_KEY` in `.env` (loaded automatically), used by the agent.
- Assets: `just pull-assets`, for the floor-photo fixtures, the fox model, and the robot-eyes face sprites.
- Optional: the kokoro `tts` command on `PATH` for spoken audio. Without it, speech is logged and skipped.

## v1: mock head and body

One process, both clients mocked: the head reads the floor photos and logs the emotion, the body logs the heading.

```sh
just perceive-act-v1
```

## v2: wgpu fox body

Same mock head, but the body is a 3d fox in a window that drives off each heading.

```sh
just perceive-act-v2
```

## v3: browser head

No in-process clients. A second HTTP server serves a wasm browser head that connects back over the socket, serving the real webcam, Web Speech, and a rendered face. Build the head wasm once, then run both servers:

```sh
just perceive-act-build-head
just perceive-act-v3
```

Both servers bind all interfaces, so any device on the LAN works: open `http://<this-host>:8337` (or `127.0.0.1` locally) and grant camera access; the head derives the agent's socket url from the page host. The `/debug` route shows the webcam, face, and log on one page. All socket clients reconnect with exponential backoff, so the page survives an agent restart.

Two ports must both be reachable from the phone: `8337` (the head page it loads first) and `8338` (the socket the head connects back to). If a device can't connect while `127.0.0.1` works locally, the host firewall is dropping inbound, not beet, since both servers already bind `0.0.0.0`. With `ufw` active (default-deny inbound), open both, scoped to the LAN subnet and tcp only (least privilege, adjust `192.168.86.0/24` to your subnet):

```sh
sudo ufw allow from 192.168.86.0/24 to any port 8337 proto tcp
sudo ufw allow from 192.168.86.0/24 to any port 8338 proto tcp
```

### Real device body (optional)

Instead of the fox, an Arduino Alvik can connect as the body and drive off each heading. From the adjacent `../beet_esp`, with the device plugged in (see that repo for toolchain and flashing details):

```sh
cargo run --release --features alvik,sockets        # flash the firmware + monitor
beet load templates/alvik/perceive-act-body.bsx     # push the body scene (BEET_REMOTE_URL -> device)
```

Set the `url` in `perceive-act-body.bsx` to this host's socket server (`ws://<host>:8338`). The body holds each `apply-heading` reply until the drive step finishes, so the next photo waits for the robot to stop; tune the step with `DriveStepConfig` in that scene. Its transport reconnects with exponential backoff, so the pushed scene survives agent restarts.
