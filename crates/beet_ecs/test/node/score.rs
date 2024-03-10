use beet_ecs::node::Score;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	expect(Score::Fail).to_be(Score::Fail)?;
	expect(Score::Fail).to_be_less_than(Score::Pass)?;
	expect(Score::Fail).to_be_less_than(Score::Weight(0.5))?;
	expect(Score::Weight(0.5)).to_be_less_than(Score::Pass)?;
	expect(Score::Weight(0.4)).to_be_less_than(Score::Weight(0.5))?;

	Ok(())
}
