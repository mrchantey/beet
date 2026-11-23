//! AWS over beet's own [`Request`]: Signature Version 4, the credential
//! sources, and one client for every protocol beet speaks.
//!
//! - [`SigV4`] signs a request in its wire form, verified against AWS's
//!   published test suite.
//! - [`AwsCredentials`] is one identity; an [`AwsCredentialSource`] answers
//!   one asynchronously and caches it until five minutes before it lapses.
//!   [`AwsDefaultChain`] is the launch's own: the environment, the shared
//!   credentials file, the ECS container endpoint, EC2 instance metadata.
//! - [`AwsAuth`] is the [`RequestAuth`] that signs as a source and recovers
//!   from a lapsed credential or a skewed clock ([`AwsClock`]).
//! - [`AwsClient`] calls one [`AwsService`] in one region: `awsJson`,
//!   `restJson1`, `restXml` and the query protocols, with retries, the S3
//!   bucket addressing rules and an [`AwsError`] parsed from whichever shape
//!   an error arrives in. [`AwsSts`] is the identity calls on top of it.
//!
//! No SDK, no async runtime and no `aws` cli: the same code signs from a
//! native binary, a Lambda and a Cloudflare Worker. The services a crate
//! calls are written beside it (`DynamoStore`, `S3Store`, the deploy verbs),
//! as request builders over [`AwsClient`], their wire shapes read from each
//! service's Smithy model.
mod auth;
mod client;
mod credential_source;
mod credentials;
mod error;
mod service;
mod sigv4;
mod sts;
pub use auth::*;
pub use client::*;
pub use credential_source::*;
pub use credentials::*;
pub use error::*;
pub use service::*;
pub use sigv4::*;
pub use sts::*;
