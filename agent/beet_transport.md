# beet_transport

How exciting! we are at the beginning of a no-std general purpose transport crate. http, stdio, websockets, gopher, smoke signals? all just ways of moving bytes around.

## The need

We're building a bevy metaframework. Bevy already has opinions about async runtimes, state management etc, and crates like `hyper` introduce non-trivial dependencies like `tokio` which are platform dependent and cut against the grain of a pure ecs application.

Bevy is cross-platform, it is no-std and happily runs on tiny devices like the esp32. Likewise the general abstraction of beet_transport should be agnostic to the target, including very constrained environments like microcontrollers and wasm.

We are building a non-tokio dependent bevy-first http server. I've included the now unmaintained `tiny-http` in `./agent/reference/tiny-http` for your reference.

This is a big task and will require careful planning, create a plan in `./agent/http-plan.md`, including dependencies, approaches, potential issues, security considerations etc. The plan should contain a detailed checklist and be implementable in stages where each stage can be verified by tests, for example:

1. basic tcp hello world
2. native tls
3. websockets..


## Features

- Accepting and managing connections to the clients
- HTTPS (`rustls-tls`, `native-tls` feature flags)
- `Transfer-Encoding` and `Content-Encoding`
- Request / Response streaming
- Connection: upgrade (used by websockets)
- Websockets
- Transport agnostic: we should use TcpListener / TcpStream for native, but eventually will need to support more strange environments like esp32 and wasm. This should have an added benefit of testability by using simple byte streams.
