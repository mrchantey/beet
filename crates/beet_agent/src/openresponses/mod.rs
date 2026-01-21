//! Implementation of the [OpenResponses Specification](https://www.openresponses.org/specification).
//!
//! This module provides Rust types for the OpenResponses API, a standardized interface
//! for generating text, executing tools, and handling multi-turn conversations with LLMs.
//!
//! # Overview
//!
//! The API consists of request and response types organized as:
//! - [`request`]: Types for constructing API requests
//! - [`response`]: Types for parsing API responses
//!
//! Common types are re-exported at the module level for convenience.
//!
//! # Example
//!
//! ```no_run
//! use beet_agent::prelude::*;
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

// Re-export common types at the module level
pub use content::*;
pub use enums::*;
pub use item::*;
pub use tool::*;
pub use usage::*;

// Re-export the main body types with clearer names
pub use request::RequestBody;
pub use response::Body as ResponseBody;

/// The default OpenResponses API endpoint for OpenAI.
pub const OPENAI_RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
