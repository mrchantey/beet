use beet_ecs::builtin_nodes::BuiltinNode;
use strum::IntoEnumIterator;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	expect(BuiltinNode::iter().count()).to_be_greater_than(0)?;

	Ok(())
}
