#[template]
fn SayHello(name: String) -> impl Bundle {
	(
		Name::new("My Behavior"),
		Sequence,
		RunOnSpawn,
		children![
			(
				Name::new("Hello"),
				EndWith(Outcome::Pass)
			),
			(
				Name::new(name),
				EndWith(Outcome::Pass)
			)
		]
	)
}