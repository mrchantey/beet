# `beet_router`

Beet Router is an ergonomic router for web frameworks.
Its a bit like an ECS alternative to `tower`, using `bevy` ecs and `beet_flow` for control flow.


The main purpose of a router is control flow, deciding what to run and when.
The recommended pattern is a behavior tree `GetOutcome` / `Outcome` but the flexible nature of ECS allows users to define their own patterns,
while still using common extractors, middleware etc.

## Servers

`beet_router` can be used without a server which is useful for testing.
For serving content the recommended approach is to use the `beet_net` server, a 100% bevy server built on `hyper`.


## Goals

`beet_router` is designed to balance performance with flexibility and developer experience.
It is not intended for extreme traffic scenarios like a proxy servers handling 10,000 requests/second, this is something axum is designed for (they go to great length to avoid a single boxing).
An average lambda request takes 200ms and our baseline target for a basic useful router with middleare is 200us, 0.1% of that.
