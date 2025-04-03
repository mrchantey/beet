use crate::prelude::*;


pub fn primary() -> RsxNode {
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
