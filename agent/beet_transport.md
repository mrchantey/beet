# beet_transport

How exciting! we are at the beginning of a no-std general purpose transport crate. http, websockets, gopher, smoke signals? all just 


## The need for another transport crate

We're building a bevy metaframework. Bevy already has mechanisms for async runtimes, for 




We are creating a new crate: `beet_transport`

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
