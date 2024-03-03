use beet_ecs::prelude::*;
use strum_macros::Display;
use strum_macros::EnumIter;
use sweet::*;

#[derive(Clone, FieldUi)]
struct TestStruct {
	#[number(min = 0, max = 100, step = 1, variant=NumberFieldVariant::default())]
	pub field1: u32,
	#[hide_ui]
	#[allow(unused)]
	pub field2: u32,
}

#[derive(Debug, PartialEq, Clone, EnumIter, Display, FieldUi)]
// #[hide_ui]
enum TestEnum {
	Variant1,
	Variant2 {
		field0: u32,
		#[hide_ui]
		field1: u32,
		field2: u32,
	},
	Variant3(u32),
}
#[derive(Debug, PartialEq, Clone, EnumIter, Display, FieldUi)]
#[hide_ui]
enum TestEnum2 {
	Variant1,
}


#[sweet_test]
pub fn works() -> Result<()> {
	let ui = FieldUiRoot::new(TestEnum::Variant1).get_ui();
	if let FieldUi::Select(_) = ui {
	} else {
		anyhow::bail!("waat");
	}
	let ui = FieldUiRoot::new(TestEnum::Variant2 {
		field0: 0,
		field1: 0,
		field2: 0,
	})
	.get_ui();

	if let FieldUi::Group(group) = ui {
		expect(group.children.len()).to_be(3)?;
	} else {
		anyhow::bail!("waat");
	}

	let ui = FieldUiRoot::new(TestEnum2::Variant1).get_ui();
	// println!("{}", ui);
	if let FieldUi::Heading(heading) = ui {
		expect(&heading.text).to_be(&"No Fields".to_string())?;
	} else {
		anyhow::bail!("waat");
	}

	Ok(())
}
#[sweet_test]
pub fn works_for_action_lists() -> Result<()> {
	let ui = FieldUiRoot::new(BuiltinNode::EmptyAction(EmptyAction)).get_ui();
	// if let FieldUi::Heading(heading) = ui {
	// 	expect(&heading.text).to_be(&"".to_string())?;
	// } else {
	// 	anyhow::bail!("waat");
	// }
	if let FieldUi::Group(group) = ui {
		expect(group.children.len()).to_be(1)?;

		if let FieldUi::Group(group) = &group.children[0] {
			expect(group.children.len()).to_be(0)?;
		} else {
			anyhow::bail!("waat");
		}
	} else {
		anyhow::bail!("waat");
	}
	Ok(())
}
