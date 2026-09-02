+++
title = "Tutorials"
+++

# Tutorials

These three lessons build one guestbook, three times over. Each starts from a complete listing you can paste, so you can begin at any of them, and each ends where the next one begins.

They are also a climb. Beet is malleable at three layers, and the guestbook meets them in order: you shape a scene by hand, you teach it new words with sandboxed scripts, and you extend the engine with compiled Rust.

- [A guestbook from a scene](/docs/tutorials/guestbook-scenes) builds a working tool out of one markup file and no code, serves it over the command line, HTTP and a terminal, and reshapes it while it runs.
- [Teach it new words](/docs/tutorials/guestbook-scripts) adds behavior the scene has no vocabulary for, in JavaScript that runs sandboxed with no authority of its own, without recompiling anything.
- [Extend the engine](/docs/tutorials/guestbook-plugins) writes a compiled action for the one thing a script cannot have, and gives the book a memory that survives a restart.

The first lesson needs only the `beet` binary. The third needs a Rust toolchain and a crate of your own.

[Self-hosted mail](/docs/mail) sits outside this section. It is an infrastructure runbook rather than a lesson: it stands up a real mail server on real cloud resources with a real domain, costs about US$43 a month to keep running, and takes a week rather than an afternoon because most of that week is spent waiting on a support case.

For material organised by feature rather than by lesson, the [examples](https://github.com/mrchantey/beet/tree/main/examples) directory covers behavior trees, routers, servers, scripting, agents and infrastructure one topic at a time.
