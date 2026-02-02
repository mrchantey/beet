//! Non-send plugin trait for Bevy.
//!
//! This module provides the [`NonSendPlugin`] trait, which is a consistent
//! interface for `!Send` types that want to behave like a [`Plugin`].

use bevy::app::App;

/// A consistent interface for `!Send` types that want to be a [`Plugin`].
///
/// Standard Bevy plugins require `Send + Sync`, which prevents using them
/// with non-send resources or types. This trait provides an alternative
/// that can be added to an app using [`BeetCoreAppExt::add_non_send_plugin`].
pub trait NonSendPlugin: Sized {
	/// Builds the plugin by configuring the app.
	fn build(self, app: &mut App);
}
