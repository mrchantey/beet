use crate::prelude::*;


pub fn get() -> RsxNode {
	rsx! {
			<h2>Variants</h2>
			<div>
				<Button>Beet is rad</Button>
				<Button variant=ButtonVariant::Secondary>Beet is rad</Button>
				<Button variant=ButtonVariant::Tertiary>Beet is rad</Button>
				<Button variant=ButtonVariant::Outlined>Beet is rad</Button>
				<Button variant=ButtonVariant::Text>Beet is rad</Button>
			</div>
			<div>
				<Button disabled>Beet is rad</Button>
				<Button disabled variant=ButtonVariant::Secondary>Beet is rad</Button>
				<Button disabled variant=ButtonVariant::Tertiary>Beet is rad</Button>
				<Button disabled variant=ButtonVariant::Outlined>Beet is rad</Button>
				<Button disabled variant=ButtonVariant::Text>Beet is rad</Button>
			</div>
			<h2>Links</h2>
			<div>
				<Link>Beet is rad</Link>
				<Link variant=ButtonVariant::Secondary>Beet is rad</Link>
				<Link variant=ButtonVariant::Tertiary>Beet is rad</Link>
				<Link variant=ButtonVariant::Outlined>Beet is rad</Link>
				<Link variant=ButtonVariant::Text>Beet is rad</Link>
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
