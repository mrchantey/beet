use crate::prelude::*;
use bevy::prelude::*;

pub fn get() -> impl Bundle {
	rsx! { <Inner client:load /> }
}

// temp until global client:load
#[template]
#[derive(Reflect)]
pub fn Inner() -> impl Bundle {
	let (value, set_value) = signal(0);
	let val2 = value.clone();
	rsx! {
			<h2>Interactivity</h2>
			<p id="interactive-text">value: {value}</p>
			<Button id="interactive-button" onclick=move|_|set_value(val2() + 1)>Increment</Button>
			<h2>Variants</h2>
			<div>
				<Button variant=ButtonVariant::Primary>		Primary 	</Button>
				<Button variant=ButtonVariant::Secondary>	Secondary </Button>
				<Button variant=ButtonVariant::Tertiary>	Tertiary 	</Button>
				<Button variant=ButtonVariant::Outlined>	Outlined 	</Button>
				<Button variant=ButtonVariant::Text>			Text 			</Button>
			</div>
			<div>
				<Button disabled variant=ButtonVariant::Primary>		Primary			</Button>
				<Button disabled variant=ButtonVariant::Secondary>	Secondary		</Button>
				<Button disabled variant=ButtonVariant::Tertiary>		Tertiary 		</Button>
				<Button disabled variant=ButtonVariant::Outlined>		Outlined 		</Button>
				<Button disabled variant=ButtonVariant::Text>				Text 				</Button>
			</div>
			<h2>Links</h2>
			<div>
				<Link variant=ButtonVariant::Primary> 	Primary		</Link>
				<Link variant=ButtonVariant::Secondary> Secondary </Link>
				<Link variant=ButtonVariant::Tertiary> 	Tertiary 	</Link>
				<Link variant=ButtonVariant::Outlined>	Outlined 	</Link>
				<Link variant=ButtonVariant::Text>			Text 			</Link>
			</div>
			<style>
			div{
				padding:1.em;
				display: flex;
				flex-direction: row;
				align-items:flex-start;
				gap: 1rem;
			}
			</style>
	}
}
