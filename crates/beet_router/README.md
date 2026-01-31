# `beet_router`

 Ergonomic IO agnostic router using Bevy ECS and behavior trees.

 `beet_router` provides a flexible routing system that uses the ECS pattern
 for request handling. Routes are defined as entity hierarchies where
 middleware, predicates, and handlers compose naturally.

 # Features

 - **ECS-based routing**: Routes are entities with components
 - **Behavior tree control flow**: Uses `beet_flow` for request routing decisions
 - **Composable middleware**: Middleware as components that can be mixed freely
 - **Type-safe extractors**: Extract typed data from requests

 # Example

 ```ignore
 use beet_router::prelude::*;
 use beet_core::prelude::*;

 let mut world = World::new();
 world.spawn(
     Endpoint::get("/hello")
         .body("Hello, World!")
 );
 ```
# Axum Comparison

`beet_router` is designed to balance performance with flexibility and developer experience.
It is not intended for extreme traffic scenarios like a proxy servers handling 10,000 requests/second, this is something axum is designed for.
An average lambda request takes 200ms and our baseline target for a relatively large router is 1ms, 0.5% of that.
