//! Object storage abstraction for S3, filesystem, and in-memory backends.
//!
//! This module provides a unified API for storing and retrieving binary data
//! across different storage backends:
//!
//! - [`InMemoryProvider`]: Ephemeral storage for testing
//! - [`FsBucketProvider`]: Local filesystem storage (native only)
//! - [`LocalStorageProvider`]: Browser localStorage (WASM only)
//! - [`S3Provider`]: AWS S3 storage (requires `aws` feature)
//! - [`DynamoDbProvider`]: AWS DynamoDB storage (requires `aws` feature)
//!
//! Use [`BucketPlugin`] to register observers that auto-insert a [`Bucket`]
//! component whenever a provider component is added to an entity.
//!
//! # Example
//!
//! ```
//! # use beet_net::prelude::*;
//! # use beet_core::prelude::*;
//! # async fn run() -> Result<()> {
//! let bucket = temp_bucket();
//! bucket.insert(&RoutePath::from("/hello.txt"), "world").await?;
//! let data = bucket.get(&RoutePath::from("/hello.txt")).await?;
//! # Ok(())
//! # }
//! ```
#[cfg(feature = "json")]
mod analytics;
#[cfg(all(not(target_arch = "wasm32"), feature = "fs"))]
mod aws_cli;
mod bucket;
#[cfg(feature = "json")]
mod table;
#[cfg(feature = "json")]
pub use analytics::*;
#[cfg(all(not(target_arch = "wasm32"), feature = "fs"))]
pub use aws_cli::*;
#[cfg(feature = "json")]
pub use table::*;
mod bucket_item;
mod in_memory_provider;
pub use bucket::*;
pub use bucket_item::*;
pub use in_memory_provider::*;
mod fs_provider;
#[cfg(target_arch = "wasm32")]
mod local_storage_provider;
pub use fs_provider::*;
#[cfg(target_arch = "wasm32")]
pub use local_storage_provider::*;
#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
pub use s3_provider::*;
#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
mod s3_provider;
#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
pub use dynamo_provider::*;
#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
mod dynamo_provider;

use beet_core::prelude::*;

/// Observer that auto-inserts a [`Bucket`] component when a provider
/// component `T` is added to an entity.
fn add_bucket<T: Component + BucketProvider + Clone>(
	ev: On<Insert, T>,
	query: Query<&T>,
	mut commands: Commands,
) -> Result {
	let entity = ev.entity;
	let provider = query.get(entity)?;
	commands
		.entity(entity)
		.insert(Bucket::new(provider.clone()));
	Ok(())
}

/// Plugin that registers bucket provider observers.
///
/// When added to an [`App`], this plugin ensures that any entity with a
/// provider component (e.g. [`FsBucketProvider`], [`InMemoryProvider`])
/// automatically receives a [`Bucket`] component wrapping that provider.
pub struct BucketPlugin;

impl Plugin for BucketPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<FsBucketProvider>()
			.add_observer(add_bucket::<FsBucketProvider>)
			.add_observer(add_bucket::<InMemoryProvider>);

		#[cfg(target_arch = "wasm32")]
		app.register_type::<LocalStorageProvider>()
			.add_observer(add_bucket::<LocalStorageProvider>);

		#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
		app.register_type::<S3Provider>()
			.register_type::<DynamoDbProvider>()
			.add_observer(add_bucket::<S3Provider>)
			.add_observer(add_bucket::<DynamoDbProvider>);
	}
}
