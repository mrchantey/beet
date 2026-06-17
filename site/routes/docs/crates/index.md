+++
title = "Crates"
+++

# Crates

Beet is a workspace of small crates, each usable on its own. The top-level `beet` crate simply re-exports them behind feature flags, so an app pulls in only the parts it needs. They layer cleanly: the agent and tooling crates are built from the core ones, not bolted onto them.

A readiness meter marks how settled each crate is:

- 🦢 ready to go, documented and tested
- 🐣 near stable, incomplete docs
- 🐉 highly experimental, here be dragons

## Core

The foundations every other crate shares.

| Crate | Status | What it does |
|---|---|---|
| [beet_core](/docs/crates/beet_core) | 🦢 | Cross-platform types, extension traits and the test runner |
| [beet_net](/docs/crates/beet_net) | 🐣 | Transport-agnostic request/response networking |
| [beet_action](/docs/crates/beet_action) | 🐣 | Entities as callable async functions |
| [beet_ui](/docs/crates/beet_ui) | 🐉 | XML-like UI trees rendered to HTML or the terminal |
| [beet_router](/docs/crates/beet_router) | 🐉 | Transport-agnostic routing for Bevy apps |
| [beet_infra](/docs/crates/beet_infra) | 🐉 | Infrastructure as code, built on OpenTofu |
| [beet_async](/docs/crates/beet_async) | 🐉 | Vendored async-world bridge for wasm and exclusive world access |

## Agents and behavior

Behaviors built on `beet_action`: behavior trees, utility AI and agentic systems.

| Crate | Status | What it does |
|---|---|---|
| [beet_thread](/docs/crates/beet_thread) | 🐉 | Multi-actor orchestration for chat, humans and agents |
| [beet_spatial](/docs/crates/beet_spatial) | 🐉 | Spatial actions: movement, steering and robotics |
| [beet_ml](/docs/crates/beet_ml) | 🐉 | Machine learning actions: embeddings and reinforcement learning |

## Apps and tooling

| Crate | Status | What it does |
|---|---|---|
| [beet-cli](/docs/crates/beet-cli) | 🐉 | Build, serve and run-wasm helpers for beet apps |

`beet_site` (this website) and `beet_examples` (shared scaffolding for the larger examples) round out the workspace. Both are built with beet but are not meant to be depended on directly.

## How they stack up

`beet_core` sits at the bottom and `beet_async` gives futures exclusive access to the Bevy world. On top of those, `beet_action` turns entities into callable functions, the primitive that `beet_net`, `beet_router`, `beet_thread`, `beet_spatial` and `beet_ml` all build on. `beet_ui` describes interfaces, and `beet_infra` describes the cloud they deploy to. Pick one crate or compose them all.
