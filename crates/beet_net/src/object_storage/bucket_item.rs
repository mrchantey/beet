use crate::prelude::*;
use beet_core::prelude::*;





#[derive(Clone)]
pub struct BucketItem {
	/// The bucket to use for getting items
	pub bucket: Bucket,
	/// The path to the item in the bucket
	pub path: RoutePath,
	pub get_data: Getter<Option<String>>,
	pub set_data: Setter<Option<String>>,
	pub get_err: Getter<Option<String>>,
	pub set_err: Setter<Option<String>>,
}

impl BucketItem {
	pub fn new(bucket: Bucket, path: RoutePath) -> Self {
		let (get_data, set_data) = signal::<Option<String>>(None);
		let (get_err, set_err) = signal::<Option<String>>(None);
		let this = Self {
			bucket,
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
	pub fn data(&self) -> Option<String> { self.get_data.get() }
	pub fn set_data(&self, data: Option<String>) { self.set_data.set(data) }
	pub fn err(&self) -> Option<String> { self.get_err.get() }
	pub fn set_err(&self, err: Option<String>) { self.set_err.set(err) }


	#[cfg_attr(not(feature = "client"), allow(unused))]
	fn init_effects(&self) {
		let this = self.clone();

		// init - get item
		effect(move || {
			let this = this.clone();
			async_ext::spawn_local(async move {
				match this.bucket.get(&this.path).await {
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
					match this.bucket.insert(&this.path, data).await {
						Ok(()) => {}
						Err(err) => this.set_err(Some(err.to_string())),
					}
				})
				.detach();
			}
		});
	}
}
