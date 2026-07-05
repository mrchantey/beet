# Perceive-act

A small floor embodied agent living a perceive-act loop: each cycle it is shown a fresh photo and answers with a single `respond-multi-modal` tool call, setting its face, saying one line in character and driving off at a chosen velocity, over and over. The loop runs until you stop it with Ctrl+C.

One model call per cycle: the `Camera` actor captures via the `take-photo` route and posts the photo straight into the thread, then the `Robot` agent (OpenAI `gpt-5.4-mini`, reasoning off, forced tool choice) answers with one `respond-multi-modal` call, which fans out to `set-emotion`/`speak-text`/`drive` (by default finishing the spoken line before it drives) and awaits them all, so the next photo waits for the body to stop moving. A cycle lands around 2 seconds plus speech.

The agent is a socket server (`agent.bsx`, shared by every version); only the head and body clients change between versions. Run every command from the repo root. For clean per-cycle output quiet the socket plumbing with `RUST_LOG=info,beet_net=warn`, which reads like:

```
cycle 3: photo captured (100KB in 0.24s, previous cycle 1.76s)
cycle 3: Surprised | "Whoa, hanging basket!" | drive lin=40 ang=-90 for 1.00s (model 1.66s)
cycle 3: acted in 0.10s (set-emotion 0.07s | speak-text 0.10s | drive 0.07s)
```

## Prerequisites

- The beet cli with the listed features, ie `cargo install --path crates/beet-cli --all-features` (each entry declares its requirements with `<CrateCheck>` and the command's `--features` verifies them, so a leaner install fails fast with the missing list).
- `OPENAI_API_KEY` in `.env` (loaded automatically), used by the agent.
- Assets: `just pull-assets`, for the floor-photo fixtures, the fox model, and the robot-eyes face sprites.
- Optional: the kokoro `tts` command on `PATH` for spoken audio. Without it, speech is logged and skipped.

## v1: mock head and body

One process, both clients mocked: the head reads the floor photos and logs the emotion, the body records the drive command.

```sh
beet --main=examples/perceive_act/main-v1.bsx
```

## v2: wgpu fox body

Same mock head, but the body is a 3d fox in a window that drives off each command.

```sh
beet --main=examples/perceive_act/main-v2.bsx
```

## v3: browser head

No in-process clients. A second HTTP server serves a wasm browser head that connects back over the socket, serving the real webcam, Web Speech, and a rendered face. Install the wasm binary once (the same single `assets/wasm/beet.wasm` every wasm example mounts), then run both servers:

```sh
beet build-wasm --release --package=beet-cli --bin=beet --features=web_examples,web_head --out=assets/wasm/beet.wasm
beet --main=examples/perceive_act/main-v3.bsx
```

Both servers bind all interfaces, so any device on the LAN works: open `https://<this-host>:8337` (or plain `http://127.0.0.1:8337` locally) and grant camera access; the head derives the agent's socket url from the page location. The `/debug` route shows the webcam, face, and log on one page. All socket clients reconnect with exponential backoff, so the page survives an agent restart.

### Phones and the webcam secure context

Browsers expose the webcam (`navigator.mediaDevices`) only in a secure context: https, `localhost` or `127.0.0.1`. A phone opening plain `http://192.168.x.x:8337` is an insecure origin, so `take-photo` fails (with remedies in the error). Both v3 servers therefore declare a `Tls` component in their scenes (`main-v3.bsx`, `agent.bsx`): a binary with the `secure` feature serves https and wss automatically, no flags.

A self-signed certificate is generated and cached in `target/tls` (regenerated when the machine's addresses change) and both ports serve TLS from it: open `https://<this-host>:8337` and accept the one-time warning. The head then connects back over `wss`. If its socket stays disconnected (Firefox and iOS scope certificate exceptions per port), visit `https://<this-host>:8338` once and accept there too; the landing page confirms it and the head reconnects on its own. TLS here is additive, not a lockout: plaintext `ws://` clients (the Alvik body) and loopback http (a `127.0.0.1` tab, the reload watcher) keep working on the same ports, since the self-signed cert exists to grant a secure context, not transport security. On managed platforms (lambda, ECS/Fargate) `Tls` detects the environment and goes inert, deferring to the platform TLS layer; `BEET_TLS=off` forces the same anywhere.

No phone involved? Use the serving machine's own browser at `http://127.0.0.1:8337`: loopback is plaintext-served and already a secure context (the face display and webcam run there; a phone can still be the display via `/` with the webcam machine on `/debug`).

### Firewalls

Two ports must both be reachable from the phone: `8337` (the head page it loads first) and `8338` (the socket the head connects back to). If a device can't connect while `127.0.0.1` works locally, the host firewall is dropping inbound, not beet, since both servers already bind `0.0.0.0`. With `ufw` active (default-deny inbound), open both, scoped to the LAN subnet and tcp only (least privilege, adjust `192.168.86.0/24` to your subnet):

```sh
sudo ufw allow from 192.168.86.0/24 to any port 8337 proto tcp
sudo ufw allow from 192.168.86.0/24 to any port 8338 proto tcp
```

### Real device body (optional)

Instead of the fox, an Arduino Alvik can connect as the body and drive off each heading. From the adjacent `../beet_esp`, with the device plugged in (see that repo for toolchain and flashing details):

```sh
# 1. flash the firmware
beet flash
# 2. load the scene
beet load templates/alvik/perceive-act-body.bsx
# 3. monitor
beet monitor
```

Set the `url` in `perceive-act-body.bsx` to this host's socket server (`ws://<host>:8338`). The body drives the commanded velocity for the commanded duration and holds each `drive` reply until the step finishes, so the next photo waits for the robot to stop; cap how long any one response may drive with `max_drive_duration` on the agent's `RespondMultiModal`. Its transport reconnects with exponential backoff, so the pushed scene survives agent restarts.
