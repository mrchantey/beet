use beet_rsx::as_beet::*;




/// https://lh3.googleusercontent.com/rfxJv95pIoJ3cEZ9ypfimJFC5Ps8sEEVBNWD36C-fy3DYvec8J_VLRosBkwTNsnpSCgSpxWXBypOXT8Ydm4fJOQ2ajWoy7SjocrzJcK7KA8=s0
pub fn light() -> RsxNode {
	rsx! {
		<div class="primary">Primary</div>
		<style>
		.primary{
			color: var(--bt-color-on-primary);
			background-color: var(--bt-color-primary);
		}
		</style>
	}
}


/// https://lh3.googleusercontent.com/S-tgf061eUWcbEBhyicTYR9PWVDeXSsSgZ2e2yYSr6Jn4W-F9z5czZCG6sv58wgJQODQakVRBDvUX5gaotfq3BuqMDLROrCO4D0Kz9F494LW=s0
pub fn dark() -> RsxNode {
	rsx! {
		<div class="primary">
		hello world
		</div>
		<style>
		</style>
	}
}
