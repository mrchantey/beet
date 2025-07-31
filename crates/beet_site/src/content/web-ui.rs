#[template]
fn Counter(initial: u32) -> impl Bundle {
	let (value, set_value) = signal(initial);
	rsx! {
		<div>
			<button onclick={move |_| set_value(value() + 1)}>
				"Count: " {value}
			</button>
		</div>
	}
}
