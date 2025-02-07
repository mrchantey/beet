mod signal;
// use crate::rsx::RsxAttribute;
// use crate::rsx::RsxNode;
// use crate::rsx::RsxRust;
use crate::prelude::*;
pub use signal::*;



/// a signals implementation of an rsx mapper
pub struct SignalsRsx;

impl SignalsRsx {
	pub fn register_block<M>(
		block: impl 'static + Clone + IntoRsx<M>,
	) -> RegisterEffect {
		Box::new(move |loc: DomLocation| {
			effect(move || {
				let block = block.clone();
				CurrentHydrator::with(move |hydrator| {
					let node = block.clone().into_rsx();
					hydrator.update_rsx_node(node, loc).unwrap()
				});
			});
			Ok(())
		})
	}
	pub fn register_attribute_block(
		&self,
		mut block: impl 'static + FnMut() -> RsxAttribute,
	) -> RegisterEffect {
		Box::new(move |loc| {
			effect(move || {
				let attrs = block();
				println!(
					"would update attributes for {}\n{}",
					loc.rsx_idx,
					RsxToHtml::default().map_attribute(&attrs).render()
				);
				todo!();
			});
			Ok(())
		})
	}
	pub fn register_attribute_value<M>(
		key: &str,
		block: impl 'static + Clone + IntoRsxAttributeValue<M>,
	) -> RegisterEffect {
		let key = key.to_string();
		Box::new(move |loc| {
			effect(move || {
				let value = block.clone().into_attribute_value();
				println!(
					"would update attribute for {}\n{key}: {value}",
					loc.rsx_idx
				);
				todo!();
			});
			Ok(())
		})
	}
}


#[cfg(test)]
mod test {
	use super::signal;
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let (get, set) = signal(7);

		let rsx = || rsx! { <div>value is {get}</div> };
		CurrentHydrator::set(HtmlNodeHydrator::new(rsx.clone()));

		rsx().register_effects();
		expect(&CurrentHydrator::with(|h| h.render()))
			.to_contain("<div data-beet-rsx-idx=\"0\">value is 7</div>");
		set(8);
		expect(&CurrentHydrator::with(|h| h.render()))
			.to_contain("<div data-beet-rsx-idx=\"0\">value is 8</div>");
		set(9);
		expect(&CurrentHydrator::with(|h| h.render()))
			.to_contain("<div data-beet-rsx-idx=\"0\">value is 9</div>");
	}
}
