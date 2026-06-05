+++
title = "Docs"
+++

# Beet

Beet is a framework for building malleable applications, software that can be reshaped by the people who use it, in the tradition of Smalltalk and HyperCard. Everything from the command line interface to a deployed web app is a [Bevy](https://bevy.org) app, and all structure and behavior is written as Entity Component System (ECS) data.

> 🚧 Beet is pre-release and under active construction. If it sounds interesting, come and say hi in the [Beetmash Discord](https://discord.gg/DcURUQCXtx).

## One world, many interfaces

A CLI, a server and a GUI differ mostly in how bytes arrive and leave. Beet describes the application once, as entities and components, and treats the interface as a matter of input and output. The same router serves a terminal, an HTTP request and an AI tool call; the same scene tree renders to HTML or the terminal. Behaviors, requests and even cloud infrastructure all live as data in one open Bevy world.

That openness is what makes an application malleable. Structure that lives as components is structure a person can inspect and reshape while it runs, rather than logic sealed inside compiled control flow. It is the quality beet borrows from Smalltalk and HyperCard: a world you can reach into and bend, not just run.

## Where to go next

- [Tutorials](/docs/tutorials) walk you through building something from scratch, start here if you are new.
- [Crates](/docs/crates) explain what each piece of beet does and how they fit together.
- [Design](/docs/design) covers the target-agnostic design system.
- [References](/docs/references) collects the reading that shapes beet's direction.

The [blog](/blog) follows the project's month-to-month development, and the per-crate [API docs](https://docs.rs/beet) cover the details.
