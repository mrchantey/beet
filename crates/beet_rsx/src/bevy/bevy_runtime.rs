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


	///
	///
	/// ```
	/// let node = rsx!{<entity runtime:bevy Transform.translation={val}/>};
	/// ```
	pub fn register_attribute_value<M>(
		key: &str,
		block: impl 'static + Clone + IntoRsxAttributeValue<M>,
	) -> RegisterEffect {
		Box::new(move |loc| todo!())
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
