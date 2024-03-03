use beet_ecs::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use sweet::*;


#[derive(Clone, FieldUi)]
struct MyAction {
	#[number(min = 0, max = 100, step = 1, variant=NumberFieldVariant::default())]
	pub health: u32,
	pub score: Score,
	pub nested: NestedAction,
}

#[derive(Clone, FieldUi, Default)]
struct NestedAction {
	pub nested_field: u32,
	#[hide_ui]
	#[allow(unused)]
	pub nested_field_hidden: u32,
	#[allow(unused)]
	#[hide_ui]
	pub nested_field_hidden2: u32,
	pub nested_field2: u32,
}


fn setup() -> FieldUiRoot<MyAction> {
	FieldUiRoot::new(MyAction {
		health: 100,
		score: Score::Pass,
		nested: NestedAction::default(),
	})
}


#[sweet_test]
pub fn sets_value() -> Result<()> {
	let root = setup();

	if let FieldUi::Group(group) = root.get_ui() {
		if let FieldUi::NumberU32(slider) = &group.children[0] {
			slider.set(50);
		} else {
			anyhow::bail!("Expected FieldUi");
		}
	} else {
		anyhow::bail!("Expected FieldUi");
	}

	expect(root.borrow().health).to_be(50)?;

	Ok(())
}


#[sweet_test]
pub fn sets_nested_value() -> Result<()> {
	let root = setup();

	if let FieldUi::Group(group) = root.get_ui() {
		if let FieldUi::Group(nested_group) = &group.children[2] {
			expect(nested_group.children.len()).to_be(2)?;
			if let FieldUi::NumberU32(nested_field) = &nested_group.children[0]
			{
				nested_field.set(50);
			} else {
				anyhow::bail!("Expected FieldUi");
			}
		} else {
			anyhow::bail!("Expected FieldUi");
		}
	} else {
		anyhow::bail!("Expected FieldUi");
	}

	expect(root.borrow().nested.nested_field).to_be(50)?;

	Ok(())
}


#[sweet_test]
pub fn calls_on_change() -> Result<()> {
	let was_called = Rc::new(RefCell::new(Score::Pass));
	let was_called2 = was_called.clone();
	let root = setup().with_on_change(move |val| {
		*was_called2.borrow_mut() = val.score;
	});

	if let FieldUi::Group(group) = root.get_ui() {
		if let FieldUi::Select(select) = &group.children[1] {
			select.set_variant_ignoring_value(Score::Fail)?;
		} else {
			anyhow::bail!("Expected Select");
		}
	} else {
		anyhow::bail!("Expected FieldUi");
	}

	expect(*was_called.borrow()).to_be(Score::Fail)?;

	Ok(())
}
#[sweet_test]
pub fn does_not_recalculates_ui() -> Result<()> {
	let was_called = Rc::new(RefCell::new(false));
	let was_called2 = was_called.clone();
	let root = setup().with_on_ui_change(move |_| {
		*was_called2.borrow_mut() = true;
	});

	if let FieldUi::Group(group) = root.get_ui() {
		if let FieldUi::Select(select) = &group.children[1] {
			select.set_variant_ignoring_value(Score::Pass)?;
		} else {
			anyhow::bail!("Expected Select");
		}
	} else {
		anyhow::bail!("Expected FieldUi");
	}
	if let FieldUi::Group(group) = root.get_ui() {
		if let FieldUi::Select(_) = &group.children[1] {
		} else {
			anyhow::bail!("Expected Select");
		}
	} else {
		anyhow::bail!("Expected FieldUi");
	}

	expect(*was_called.borrow()).to_be_false()?;

	Ok(())
}
#[sweet_test]
pub fn recalculates_ui() -> Result<()> {
	let was_called = Rc::new(RefCell::new(false));
	let was_called2 = was_called.clone();
	let root = setup().with_on_ui_change(move |_| {
		*was_called2.borrow_mut() = true;
	});

	if let FieldUi::Group(group) = root.get_ui() {
		if let FieldUi::Select(select) = &group.children[1] {
			select.set_variant_ignoring_value(Score::Weight(10))?;
		} else {
			anyhow::bail!("Expected Select");
		}
	} else {
		anyhow::bail!("Expected FieldUi");
	}
	if let FieldUi::Group(group) = root.get_ui() {
		if let FieldUi::Group(_) = &group.children[1] {
			// pass
		} else {
			anyhow::bail!("Expected Group");
		}
	} else {
		anyhow::bail!("Expected FieldUi");
	}

	expect(*was_called.borrow()).to_be_true()?;

	// note that the actual value is not set, it just updates the ui
	expect(root.borrow().score).not().to_be(Score::Weight(10))?;

	Ok(())
}
