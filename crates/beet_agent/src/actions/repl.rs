use beet_core::prelude::*;
use beet_flow::prelude::*;


pub fn repl() -> impl Bundle {
	(Sequence, Retrigger::default(), children![
		user_input(),
		// user says,
		// agent says
	])
}


fn user_input() -> impl Bundle {
	OnSpawn::observe(|ev: On<GetOutcome>, mut commands: Commands| -> Result {
		let mut input = String::new();
		print!("User > ");
		std::io::Write::flush(&mut std::io::stdout())?;
		std::io::stdin().read_line(&mut input)?;
		println!("{}", input);
		commands.entity(ev.target()).trigger_target(Outcome::Pass);
		Ok(())
	})
}
