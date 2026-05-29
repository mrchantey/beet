//! Reactive store item for UI-driven storage access.
use crate::prelude::*;
use beet_core::prelude::*;





/// A reactive wrapper around a store path for UI-driven storage access.
///
/// Provides signal-based getters and setters that automatically sync with
/// the underlying store storage via effects.
#[derive(Clone)]
pub struct StoreItem {
	/// The store containing this item.
	pub store: BlobStore,
	/// The path to this item within the store.
	pub path: SmolPath,
	/// Getter for the item's data content.
	pub get_data: Getter<Option<String>>,
	/// Setter for the item's data content.
	pub set_data: Setter<Option<String>>,
	/// Getter for any error that occurred during operations.
	pub get_err: Getter<Option<String>>,
	/// Setter for error state.
	pub set_err: Setter<Option<String>>,
}

impl StoreItem {
	/// Creates a new store item and initializes reactive effects.
	pub fn new(store: BlobStore, path: SmolPath) -> Self {
		let (get_data, set_data) = signal::<Option<String>>(None);
		let (get_err, set_err) = signal::<Option<String>>(None);
		let this = Self {
			store,
			path,
			get_data,
			set_data,
			get_err,
			set_err,
		};

		#[cfg(feature = "client")]
		this.init_effects();

		this
	}
	/// Returns the current data content, if loaded.
	pub fn data(&self) -> Option<String> { self.get_data.get() }
	/// Sets the data content, triggering a store insert.
	pub fn set_data(&self, data: Option<String>) { self.set_data.set(data) }
	/// Returns any error that occurred during the last operation.
	pub fn err(&self) -> Option<String> { self.get_err.get() }
	/// Sets the error state.
	pub fn set_err(&self, err: Option<String>) { self.set_err.set(err) }


	#[cfg_attr(not(feature = "client"), allow(unused))]
	fn init_effects(&self) {
		let this = self.clone();

		// init - get item
		effect(move || {
			let this = this.clone();
			async_ext::spawn_local(async move {
				match this.store.get(&this.path).await {
					Ok(data) => {
						let data = String::from_utf8_lossy(&data).to_string();
						this.set_data(Some(data))
					}
					Err(err) => this.set_err(Some(err.to_string())),
				}
			})
			.detach();
		});

		let this = self.clone();
		// insert item
		effect(move || {
			let this = this.clone();
			if let Some(data) = this.data() {
				async_ext::spawn_local(async move {
					match this.store.insert(&this.path, data).await {
						Ok(()) => {}
						Err(err) => this.set_err(Some(err.to_string())),
					}
				})
				.detach();
			}
		});
	}
}
