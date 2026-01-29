//! Implementation of the [OpenResponses Specification](https://www.openresponses.org/specification).
//!
//! OpenResponses is an open-source specification for building **multi-provider,
//! interoperable LLM interfaces**. It defines a shared schema for calling language
//! models, streaming results, and composing agentic workflows—independent of provider.
//!
//! While OpenResponses provides an [openapi spec](https://www.openresponses.org/openapi/openapi.json),
//! we do not autogenerate these types, and instead use language models to generate and maintain them
//! for higher quality documentation and rusty ergonomics.
//!
//! # Key Concepts
//!
//! ## Items
//!
//! **Items** are the fundamental unit of context in OpenResponses. They represent
//! atomic units of model output, tool invocation, or reasoning state. Items are
//! bidirectional—they can be provided as inputs or received as outputs.
//!
//! Each item type has a defined schema discriminated by the `type` field:
//! - [`OutputItem::Message`]: Text messages from the model
//! - [`OutputItem::FunctionCall`]: Tool invocations with arguments
//! - [`OutputItem::Reasoning`]: Model reasoning traces (for reasoning models)
//!
//! ## Streaming
//!
//! When `stream: true` is set, responses arrive as Server-Sent Events (SSE).
//! Events are either **state transitions** (`response.completed`) or **deltas**
//! (`response.output_text.delta`). See [`streaming`] for all 24 event types.
//!
//! ## Agentic Loop
//!
//! The model can reason, invoke tools, and generate responses in a loop. For
//! externally-hosted tools (functions), control yields back to the developer
//! who must execute the function and return results in a follow-up request.
//!
//! # Module Organization
//!
//! - [`request`]: Types for constructing API requests
//! - [`response`]: Types for parsing API responses
//! - [`streaming`]: Types for Server-Sent Event streaming
//!
//! Common types are re-exported at the module level for convenience.
//!
//! # Example: Non-Streaming Request
//!
//! ```no_run
//! use beet_clanker::prelude::*;
//! use beet_core::prelude::*;
//! use beet_net::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! dotenv::dotenv().ok();
//!
//! let body = openresponses::RequestBody::new("gpt-4o-mini")
//!     .with_input("Say hello in exactly 3 words.");
//!
//! let response = Request::post(openresponses::OPENAI_RESPONSES_URL)
//!     .with_auth_bearer(&env_ext::var("OPENAI_API_KEY")?)
//!     .with_json_body(&body)?
//!     .send()
//!     .await?
//!     .into_result()
//!     .await?
//!     .json::<openresponses::ResponseBody>()
//!     .await?;
//!
//! assert_eq!(response.status, openresponses::response::Status::Completed);
//! println!("Response: {}", response.first_text().unwrap_or_default());
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Streaming Request
//!
//! ```no_run
//! use beet_clanker::prelude::*;
//! use beet_core::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! let mut provider = OllamaProvider::default();
//!
//! let body = openresponses::RequestBody::new(provider.default_small_model())
//!     .with_input("Write a haiku.")
//!     .with_stream(true);
//!
//! let stream = provider.stream(body).await?;
//! beet_core::exports::futures_lite::pin!(stream);
//!
//! while let Some(event) = stream.next().await {
//!     match event? {
//!         openresponses::StreamingEvent::OutputTextDelta(ev) => {
//!             print!("{}", ev.delta);
//!         }
//!         openresponses::StreamingEvent::ResponseCompleted(_) => break,
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```

mod content;
mod enums;
mod item;
mod tool;
mod usage;

/// Request types for the OpenResponses API.
pub mod request;

/// Response types for the OpenResponses API.
pub mod response;

/// Streaming event types for the OpenResponses API.
pub mod streaming;

// Re-export common types at the module level
pub use content::*;
pub use enums::*;
pub use item::*;
pub use streaming::StreamingEvent;
pub use tool::*;
pub use usage::*;

// Re-export the main body types with clearer names
pub use request::RequestBody;
pub use response::Body as ResponseBody;

/// The default OpenResponses API endpoint for OpenAI.
pub const OPENAI_RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
