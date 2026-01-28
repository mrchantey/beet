//! Tool types for agent tool calling.
//!
//! This module provides the relationship types for connecting agents to their
//! available tools. Each agent can have a `Tools` relationship pointing to
//! tool entities that define available functions the model can call.
//!
//! # Architecture
//!
//! Tools work similarly to [`ThreadContext`](crate::prelude::ThreadContext):
//! - Each agent gets a `Tools` / `ToolOf` relationship
//! - Each `ToolOf` entity is a `router_exchange` with its own `EndpointTree`
//! - When exposing tools to the model API, we prefix route names with the entity ID
//! - When executing tools, we use `Entity::from_bits` to find the right tool set
//!
//! # Example
//!
//! ```ignore
//! world.spawn((
//!     Agent,
//!     related![Tools,
//!         tool_exchange((Sequence, children![
//!             EndpointBuilder::post()
//!                 .with_path("add")
//!                 .with_request_body(BodyMeta::json::<AddRequest>())
//!                 .with_response_body(BodyMeta::json::<AddResponse>())
//!                 .with_description("Add two numbers together")
//!                 .with_action(|Json(req): Json<AddRequest>| {
//!                     Json(AddResponse { result: req.a + req.b })
//!                 }),
//!         ]))
//!     ],
//!     related![Children,
//!         Action1,
//!         Action2,
//!     ]
//! ));
//! ```

use beet_core::prelude::*;

/// Points to the agent that owns this tool set.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = Tools)]
pub struct ToolOf(pub Entity);

/// An ordered collection of tool sets this agent has access to.
/// Each tool set entity should have an `EndpointTree` component
/// describing the available tools.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = ToolOf, linked_spawn)]
pub struct Tools(Vec<Entity>);
