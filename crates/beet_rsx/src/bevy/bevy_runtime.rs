use crate::prelude::*;

pub struct BevyRuntime;







impl BevyRuntime {
	pub fn register_block<M>(
		_block: impl 'static + Clone + IntoRsx<M>,
	) -> RegisterEffect {
		todo!()
	}
	pub fn register_attribute_block(
		mut _block: impl 'static + FnMut() -> RsxAttribute,
	) -> RegisterEffect {
		todo!()
	}


	/// Used by [`RstmlToRsx`] when it encounters an attribute with a block value:
	/// ```
	/// # use beet_rsx::prelude::*;
	/// let value = 3;
	/// let node = rsx!{<el key={value}/>};
	/// ```
	pub fn parse_attribute_value<M>(
		key: &'static str,
		tracker: RustyTracker,
		block: impl 'static + Clone + IntoRsxAttributeValue<M>,
	) -> RsxAttribute {
		RsxAttribute::BlockValue {
			key: key.to_string(),
			initial: block.clone().into_attribute_value(),
			effect: Effect::new(
				Box::new(|_loc| {
					todo!();
				}),
				tracker,
			),
		}
	}
}



#[cfg(test)]
mod test {
	// use crate::prelude::*;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		// expect(true).to_be_false();
	}
}
