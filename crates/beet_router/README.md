# `beet_router`

Beet Router is an ECS alternative to `tower`, using `bevy` ecs and `beet_flow` for control flow.

The most common pattern is a behavior tree but the flexible nature of ECS allows users to define their own patterns,
while still getting access to common extractors, middleware etc.

## Servers

`beet_router` can be used without a server which is useful for testing.
For serving content the recommended approach is to use the `beet_net` server, a 100% bevy server built on `hyper`.
