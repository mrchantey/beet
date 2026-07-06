# Perceive-act v3 hardening

Harden the v3 perceive-act loop (`examples/perceive_act/main-v3.bsx`) so the browser head and the ESP32 Alvik body connect and drive the loop reliably from any startup ordering, and reconnect cleanly across server restarts, browser reloads and robot power cycles. This skill is the test matrix, the verification method, and the enabling changes. It is meant to be executed end to end on real hardware and a real browser, not just in unit tests.

## System model

Three actors, one host machine plus one robot:

1. `S` (server): one `beet --main=examples/perceive_act/main-v3.bsx` process running two servers in one binary. The agent `SocketServer` on `:8338` (the loop and the `respond-multi-modal` fan-out) and the head `HttpServer` on `:8337` (serves the wasm head page + assets). Both bind `0.0.0.0`, both wrap `Tls` (https/wss from a cached self-signed cert in `target/tls`, loopback http still served).
2. `H` (head): a browser tab at `https://<host>:8337` (or `http://127.0.0.1:8337` locally). Boots the wasm `beet` head, which dials back to the agent socket as the `head` client and serves `take-photo` (webcam), `speak-text` (Web Speech), `set-emotion` (the `<img id="face">`).
3. `B` (body): the ESP32 Alvik running the firmware from `../beet_esp` with the `perceive-act-body` scene. Dials back to the agent socket as the `body` client and serves `drive`.

Connection semantics (the reason ordering should not matter):

4. `S` is the socket server; `H` and `B` are `PersistentSocket` clients that redial with exponential backoff (250ms doubling to a 10s ceiling). So a client that starts before `S`, or outlives an `S` restart, keeps retrying and connects once `S` is up. No re-push, no reload required.
5. On each accepted connection `S` originates a `whoami` request and binds that role's capability routes to forward over the socket (`head` -> take-photo/speak-text/set-emotion, `body` -> drive). Re-binding happens per connection, so a reconnect rebinds automatically.
6. A bound client dropping inserts `ResetOnDisconnect` on the server side and triggers a `ResetScene`. On `B`, `ResetScene` halts the robot (motors, wheels, LEDs to rest) before its redial loop takes over, so it never holds a stale drive command while `S` is gone.
7. With no head bound, `take-photo` falls back to its local handler (the host reads the floor-photo fixtures), so the loop keeps cycling even with the browser closed. With no body bound, `drive` falls back to `RecordDrive`. Either client is independently optional.

## Enabling changes (implemented)

These remove the variables that make the permutation matrix flaky or expensive. All three are landed and verified on hardware.

8. Mock model provider (socket-only testing without OpenAI spend). `MockPostStreamer` (`crates/beet_thread/src/providers/mock_provider.rs`) gained a `tool_arguments: Vec<String>` field that, when set, is cycled (one per call, looping) as the tool call's arguments instead of schema defaults. A perceive-act `<RobotStreamer>` template (`crates/beet_extra/src/perceive_act/robot_streamer.rs`) replaces the inline `<ModelStreamer provider="OpenAi"/>` on the `Robot` actor in the shared `agent.bsx`: with `BEET_MOCK_MODEL` set it inserts the mock preloaded with a canned loop of `respond-multi-modal` payloads (swinging emotions, gentle drives, short lines), otherwise the real OpenAI streamer. So `BEET_MOCK_MODEL=1 beet --main=main-v3.bsx` runs the whole fan-out (set-emotion + speak-text + drive over the sockets) with zero network calls, and the same one scene runs real by dropping the env. The mock returns instantly, so with no body bound the loop spins fast (good socket stress); once the body binds it is paced by the drive settle, like the real loop.
9. Firmware default scene (removes the "load the scene after every power-on" failure point). `beet_esp/src/extra/default_scene.rs` embeds a `.bsx` via `include_str!(env!("BEET_DEFAULT_SCENE_FILE"))` and `DefaultScenePlugin` loads it once, after the scene server's `RouteTree` exists, through the same `set_scene` path `/load` uses, marked `BeetSceneRoot`. `build.rs`'s `embed_default_scene` resolves the `BEET_DEFAULT_SCENE` build env (crate-relative or absolute, default `templates/alvik/perceive-act-body.bsx`) to an absolute path and `rerun-if-changed`s it. A later `beet load` replaces it (set_scene despawns the prior `BeetSceneRoot`) and `beet clear` clears it for good, so the override path (a dead host, a new host at a different IP, no reflash) still works. Verified on boot: `loaded default scene: 1 root(s)`.
10. Agent URL fixed. `templates/alvik/perceive-act-body.bsx` hardcoded `wss://192.168.86.220:8338` while this host is `192.168.86.221` (off by one), which alone stopped `B` connecting; corrected to `.221`. The URL travels in the same `.bsx` that is both embedded (default) and `beet load`-able, so a new host means editing the scene's `url` (or pointing `BEET_SOCKET_SERVER` at build time) and re-pushing. A zero-IP mDNS body (firmware mDNS browser at `src/net/mdns_browser.rs`, beet_net advertise at `crates/beet_net/src/mdns/`) remains a future option.

## Permutation matrix

`S`, `H`, `B` are start events; each actor may be fresh or left running from a prior case. The invariant under test: once `S` is up, both `H` and `B` connect and the loop cycles, regardless of order or prior state; and after `S` stops and restarts, both reconnect on their own. Run 11 to 16 on the mock provider (fast, free), then 17 on real OpenAI.

11. `S` then `B` then `H`. Baseline. Each joins as it starts.
12. `S` then `H` then `B`. Head before body.
13. `B` then `S` then `H`. Body waits for `S`, connects when `S` boots, then head joins.
14. `H` then `S` then `B`. Browser open and retrying before `S` exists, connects on `S` boot, then body.
15. `B` then `H` then `S`. Both clients up and retrying, both connect the moment `S` starts (the "fresh or from a previous run, both connect once the server starts" case).
16. `H` then `B` then `S`. As 15, reversed client order.
17. Restart cycle, server. Bring `S`, `H`, `B` up, then stop `S`. Verify `B` halts (ResetScene, motors stop) and holds, `H` shows its socket disconnected/retrying and its face frozen, the loop stops. Start `S` again: both reconnect, the loop resumes. This is the headline "server stops, robot stays on (motors stop), server starts again" case.
18. Restart cycle, server, repeated. As 17 but stop/start `S` three times in quick succession to stress the backoff and the per-connection rebind. No stuck disconnect, no double-bind, loop resumes each time.
19. Reload cycle, head. `S`, `H`, `B` up, reload the browser tab (the "browser stays on, server restarts" analogue for the head). `H` reconnects, rebinds `head`, loop resumes. While `H` is gone the loop keeps cycling on the local floor-photo fallback.
20. Power cycle, body. `S`, `H`, `B` up, reset or power-cycle the ESP32. `B` redials, rebinds `body`, resumes driving. `S` and `H` unaffected.
21. Headless (no browser). `S` and `B` only. `take-photo` falls back to the host floor photos, the loop runs, `drive` reaches `B`. Proves the body path independent of the browser.
22. Bodiless (no robot). `S` and `H` only. `drive` falls back to `RecordDrive`, the loop runs, face and speech land on `H`. Proves the head path independent of the robot.

## Verification signals

Assert on all three actors per case, not just the server.

23. `S` stdout (run with `RUST_LOG=info,beet_net=warn` for clean per-cycle lines):
	- `cycle N: photo captured (…KB in …s, previous cycle …s)`
	- `cycle N: <Emotion> | "<line>" | drive lin=… ang=… for …s (model …s)`
	- `cycle N: acted in …s (set-emotion …s | speak-text …s | drive …s)`
	- monotonically increasing `cycle N` is the liveness signal; a stall means a capability is not being served.
24. `H` via Playwright (available: CLI 1.61.1, chromium 1228 cached, plus system chromium/chrome):
	- launch chromium with `--use-fake-device-for-media-stream --use-fake-ui-for-media-stream` (synthetic cam, auto-granted) or `--use-file-for-fake-video-capture=<floor>.y4m` to feed a real floor photo; context `ignoreHTTPSErrors: true`, `permissions: ['camera']` for the self-signed https.
	- navigate `https://<host>:8337` for the face display, `https://<host>:8337/debug` for the webcam + face + log panel.
	- assert the socket is connected (WebHead), the `<img id="face">` `src` changes across cycles (set-emotion landing), and the webcam `<video>` has live frames (`readyState`/`videoWidth`). On the mock, the face src still updates (to the default/canned emotion) so connectivity is provable even without OpenAI.
25. `B` via probe-rs RTT (`beet monitor`, or `cargo run --release --features alvik,sockets,secure` from `../beet_esp`):
	- `socket connected to '…'` on connect, redial/backoff lines while `S` is down.
	- `drive: (X mm/s, Y deg/s) for …` per command, `drive step complete, halting` between commands.
	- `ResetScene` / halt on disconnect. Motors are off for the talk but the wheel driver still logs, so responsiveness is observable without motion.

## Run procedure

26. One-time environment:
	- build the wasm head: `beet build-wasm --release --package=beet-cli --bin=beet --features=web_examples,web_head --out=assets/wasm/beet.wasm` (artifact is missing until built).
	- assets are present (`assets/floor-photos`, `assets/extra/robot-eyes`); if not, `just pull-assets`.
	- `OPENAI_API_KEY` is set in `beet/.env` (real pass only).
	- if `ufw` is active, open the two ports to the LAN subnet: `sudo ufw allow from 192.168.86.0/24 to any port 8337 proto tcp` and `…port 8338 proto tcp`. Loopback `127.0.0.1` needs no rule. Verify with `sudo ufw status`.
	- ESP toolchain is set up (`. $HOME/export-esp.sh`, espup `esp` channel, probe-rs with the udev rule; the device enumerates as `ESP JTAG 303a:1001`). Flash from `../beet_esp`, budget ~2min for a fresh flash.
27. Harness scripts (under `.agents/tmp`, they log there too): `pa_head_harness.js <url> <seconds> [floor.y4m]` opens the browser head with a fake webcam and logs face changes + the head's console (connect/reconnect) for the window, staying open so `S` can be cycled underneath it; `run_matrix.sh` orchestrates all three actors across a server restart (attaches the esp RTT monitor, starts/stops `S`, launches `H`, prints an evidence summary); `test_head_reconnect.sh` isolates the head reconnect; `run_real.sh` is the real-OpenAI pass feeding `floor.y4m` (built with `ffmpeg -loop 1 -i <floor>.jpg -t 4 -r 10 -vf scale/crop=640:480 -pix_fmt yuv420p -f yuv4mpegpipe`). Use `http://127.0.0.1:8337/debug` for the head: loopback is a secure context (webcam works) and needs no cert, and the head derives `ws://127.0.0.1:8338` from it. Re-monitor the esp without reflashing via `probe-rs attach --chip esp32s3 <elf>`; `probe-rs reset --chip esp32s3` simulates a power cycle.
28. Pass criteria: every case reaches a steady loop with all three signals live; every restart/reload/power case halts safely and then resumes without manual re-push or reload; the robot never holds a stale drive after `S` disappears; no permutation deadlocks on a missing capability.

## Known failure modes

29. WASM head did not reconnect after an agent restart (found + fixed). `connect_wasm` (`crates/beet_net/src/sockets/impl_web_sys.rs`) resolved its connect only on the WebSocket `open` event; a dial against a down server (the restart gap) fires `error`/`close` without `open`, so the connect future hung forever and the head's `PersistentSocket` never redialed, staying dead even after `S` returned (the esp reconnected fine, since the native transport returns `Err` on a failed dial). Fixed to settle the connect on `open` (Ok) or an `error`/`close`-before-open (Err), so a failed dial redials with backoff. Regression guard: `test_head_reconnect.sh` (expect a second `socket connected` after the restart). A wasm unit test is impractical (needs a browser + a stoppable ws server), so the browser script is the guard.
30. Stale agent IP in the body scene (see 10): a likely cause of "the body never connects". Verify the host's current IP against the scene's `url` before every session.
31. Certificate scoping: Firefox and iOS scope the self-signed exception per port, so a head may load on `:8337` but fail to reach `wss://…:8338`; visit `https://<host>:8338` once to accept. Playwright's `ignoreHTTPSErrors` sidesteps this for automation.
32. Insecure origin webcam: a phone on plain `http://192.168.x.x:8337` gets no `navigator.mediaDevices`, so `take-photo` fails. Use https (the `Tls` component) or `127.0.0.1`.
33. ESP sticky download mode: a flash that verifies but streams no RTT is a chip stuck in download mode (looks identical to dead hardware). Cold-boot (unplug ~10s, keep `COM` out, replug `USB`) before blaming a peripheral. Keep the `COM` UART port unplugged while probe-rs drives the native `USB` JTAG.
34. Server-side ResetScene on head drop is harmless on the host (the loop template is not a `BeetSceneRoot`, so it is not despawned) but is worth confirming does not tear down the running thread when a browser closes mid-loop.
