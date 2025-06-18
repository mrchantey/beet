use crate::prelude::*;



/// https://lh3.googleusercontent.com/rfxJv95pIoJ3cEZ9ypfimJFC5Ps8sEEVBNWD36C-fy3DYvec8J_VLRosBkwTNsnpSCgSpxWXBypOXT8Ydm4fJOQ2ajWoy7SjocrzJcK7KA8=s0
/// https://lh3.googleusercontent.com/S-tgf061eUWcbEBhyicTYR9PWVDeXSsSgZ2e2yYSr6Jn4W-F9z5czZCG6sv58wgJQODQakVRBDvUX5gaotfq3BuqMDLROrCO4D0Kz9F494LW=s0
pub fn get() -> impl Bundle {
	rsx! {
		<h2>Color Scheme - Light</h2>
		<div class=ThemeToCss::DEFAULT_LIGHT_CLASS>{scheme()}</div>
		<h2>Color Scheme - Dark</h2>
		<div class=ThemeToCss::DEFAULT_DARK_CLASS>{scheme()}</div>
	}
}




/// display a scheme, using the background color of divs as their
fn scheme() -> impl Bundle {
	rsx! {
		<h3>"Primary"</h3>
		<div class="color-group">
			<div class="color-box primary">Primary</div>
			<div class="color-box on-primary">On Primary</div>
			<div class="color-box primary-container">Primary Container</div>
			<div class="color-box on-primary-container">On Primary Container</div>
			<div class="color-box inverse-primary">Inverse Primary</div>
		</div>
		<div class="color-group">
		  <div class="color-box primary-fixed">Primary Fixed</div>
		  <div class="color-box primary-fixed-dim">Primary Fixed Dim</div>
		  <div class="color-box on-primary-fixed">On Primary Fixed</div>
		  <div class="color-box on-primary-fixed-variant">On Primary Fixed Variant</div>
		</div>
		<h3>"Secondary"</h3>
		<div class="color-group">
		<div class="color-box secondary">Secondary</div>
		<div class="color-box on-secondary">On Secondary</div>
		<div class="color-box secondary-container">Secondary Container</div>
		<div class="color-box on-secondary-container">On Secondary Container</div>
		</div>
		<div class="color-group">
		  <div class="color-box secondary-fixed">Secondary Fixed</div>
		  <div class="color-box secondary-fixed-dim">Secondary Fixed Dim</div>
		  <div class="color-box on-secondary-fixed">On Secondary Fixed</div>
		  <div class="color-box on-secondary-fixed-variant">On Secondary Fixed Variant</div>
		</div>
		<h3>"Tertiary"</h3>
		<div class="color-group">
			<div class="color-box tertiary">Tertiary</div>
			<div class="color-box on-tertiary">On Tertiary</div>
			<div class="color-box tertiary-container">Tertiary Container</div>
			<div class="color-box on-tertiary-container">On Tertiary Container</div>
			</div>
		<div class="color-group">
		  <div class="color-box tertiary-fixed">Tertiary Fixed</div>
		  <div class="color-box tertiary-fixed-dim">Tertiary Fixed Dim</div>
		  <div class="color-box on-tertiary-fixed">On Tertiary Fixed</div>
		  <div class="color-box on-tertiary-fixed-variant">On Tertiary Fixed Variant</div>
		</div>
		<h3>"Error"</h3>
		<div class="color-group">
		  <div class="color-box error">Error</div>
		  <div class="color-box on-error">On Error</div>
		  <div class="color-box error-container">Error Container</div>
		  <div class="color-box on-error-container">On Error Container</div>
		</div>
		<h3>"Surface"</h3>
		<div class="color-group">
			<div class="color-box surface-dim">Surface Dim</div>
			<div class="color-box surface">Surface</div>
			<div class="color-box surface-bright">Surface Bright</div>
			<div class="color-box on-surface">On Surface</div>
		</div>
		<div class="color-group">
			<div class="color-box surface-variant">Surface Variant</div>
			<div class="color-box on-surface-variant">On Surface Variant</div>
			<div class="color-box inverse-surface">Inverse Surface</div>
			<div class="color-box inverse-on-surface">Inverse On Surface</div>
		</div>
		<div class="color-group">
		  <div class="color-box surface-container-lowest">Surface Container Lowest</div>
		  <div class="color-box surface-container-low">Surface Container Low</div>
		  <div class="color-box surface-container">Surface Container</div>
		  <div class="color-box surface-container-high">Surface Container High</div>
		  <div class="color-box surface-container-highest">Surface Container Highest</div>
		</div>
		<h3>"Misc"</h3>
		<div class="color-group">
			<div class="color-box scrim">Scrim</div>
			<div class="color-box shadow">Shadow</div>
			<div class="color-box outline">Outline</div>
			<div class="color-box outline-variant">Outline Variant</div>
		</div>
		<style>
			.color-group {
			  display: flex;
			  flex-direction: row;
				gap: 2px;
				flex: 1;
			}

			.color-desc {
				padding:0.3.em;
			}
			.color-box {
				display: flex;
				align-items: center;
				padding:0.3.em;
				height:3.em;
				width:10.em;
			}

			/* First row */
			.primary {
			  background-color: var(--bt-color-primary);
			  color: var(--bt-color-on-primary);
			}
			.on-primary {
				background-color: var(--bt-color-on-primary);
				color: var(--bt-color-primary);
			}


			.secondary {
			  background-color: var(--bt-color-secondary);
			  color: var(--bt-color-on-secondary);
			}
			.tertiary {
			  background-color: var(--bt-color-tertiary);
			  color: var(--bt-color-on-tertiary);
			}
			.error {
			  background-color: var(--bt-color-error);
			  color: var(--bt-color-on-error);
			}

			/* Second row */
			.on-secondary {
			  background-color: var(--bt-color-on-secondary);
			  color: var(--bt-color-secondary);
			}
			.on-tertiary {
			  background-color: var(--bt-color-on-tertiary);
			  color: var(--bt-color-tertiary);
			}
			.on-error {
			  background-color: var(--bt-color-on-error);
			  color: var(--bt-color-error);
			}

			/* Third row */
			.primary-container {
			  background-color: var(--bt-color-primary-container);
			  color: var(--bt-color-on-primary-container);
			}
			.secondary-container {
			  background-color: var(--bt-color-secondary-container);
			  color: var(--bt-color-on-secondary-container);
			}
			.tertiary-container {
			  background-color: var(--bt-color-tertiary-container);
			  color: var(--bt-color-on-tertiary-container);
			}
			.error-container {
			  background-color: var(--bt-color-error-container);
			  color: var(--bt-color-on-error-container);
			}

			/* Fourth row */
			.on-primary-container {
			  background-color: var(--bt-color-on-primary-container);
			  color: var(--bt-color-primary-container);
			}
			.on-secondary-container {
			  background-color: var(--bt-color-on-secondary-container);
			  color: var(--bt-color-secondary-container);
			}
			.on-tertiary-container {
			  background-color: var(--bt-color-on-tertiary-container);
			  color: var(--bt-color-tertiary-container);
			}
			.on-error-container {
			  background-color: var(--bt-color-on-error-container);
			  color: var(--bt-color-error-container);
			}

			/* Fifth row (split boxes) */
			.primary-fixed {
			  background-color: var(--bt-color-primary-fixed);
			  color: var(--bt-color-on-primary-fixed);
			}
			.primary-fixed-dim {
			  background-color: var(--bt-color-primary-fixed-dim);
			  color: var(--bt-color-on-primary-fixed);
			}
			.secondary-fixed {
			  background-color: var(--bt-color-secondary-fixed);
			  color: var(--bt-color-on-secondary-fixed);
			}
			.secondary-fixed-dim {
			  background-color: var(--bt-color-secondary-fixed-dim);
			  color: var(--bt-color-on-secondary-fixed);
			}
			.tertiary-fixed {
			  background-color: var(--bt-color-tertiary-fixed);
			  color: var(--bt-color-on-tertiary-fixed);
			}
			.tertiary-fixed-dim {
			  background-color: var(--bt-color-tertiary-fixed-dim);
			  color: var(--bt-color-on-tertiary-fixed);
			}

			/* Sixth row */
			.on-primary-fixed {
			  background-color: var(--bt-color-on-primary-fixed);
			  color: var(--bt-color-primary-fixed);
			}
			.on-secondary-fixed {
			  background-color: var(--bt-color-on-secondary-fixed);
			  color: var(--bt-color-secondary-fixed);
			}
			.on-tertiary-fixed {
			  background-color: var(--bt-color-on-tertiary-fixed);
			  color: var(--bt-color-tertiary-fixed);
			}

			/* Seventh row */
			.on-primary-fixed-variant {
			  background-color: var(--bt-color-on-primary-fixed-variant);
			  color: var(--bt-color-primary-fixed);
			}
			.on-secondary-fixed-variant {
			  background-color: var(--bt-color-on-secondary-fixed-variant);
			  color: var(--bt-color-secondary-fixed);
			}
			.on-tertiary-fixed-variant {
			  background-color: var(--bt-color-on-tertiary-fixed-variant);
			  color: var(--bt-color-tertiary-fixed);
			}

			/* Surface row */
			.surface-dim {
			  background-color: var(--bt-color-surface-dim);
			  color: var(--bt-color-on-surface);
			}
			.surface {
			  background-color: var(--bt-color-surface);
			  color: var(--bt-color-on-surface);
			}
			.surface-bright {
			  background-color: var(--bt-color-surface-bright);
			  color: var(--bt-color-on-surface);
			}
			.inverse-surface {
			  background-color: var(--bt-color-inverse-surface);
			  color: var(--bt-color-inverse-on-surface);
			}

			/* Inverse On Surface row */
			.inverse-on-surface {
			  background-color: var(--bt-color-inverse-on-surface);
			  color: var(--bt-color-inverse-surface);
			}

			/* Surface containers row */
			.surface-container-lowest {
			  background-color: var(--bt-color-surface-container-lowest);
			  color: var(--bt-color-on-surface);
			}
			.surface-container-low {
			  background-color: var(--bt-color-surface-container-low);
			  color: var(--bt-color-on-surface);
			}
			.surface-container {
			  background-color: var(--bt-color-surface-container);
			  color: var(--bt-color-on-surface);
			}
			.surface-container-high {
			  background-color: var(--bt-color-surface-container-high);
			  color: var(--bt-color-on-surface);
			}
			.surface-container-highest {
			  background-color: var(--bt-color-surface-container-highest);
			  color: var(--bt-color-on-surface);
			}
			.inverse-primary {
			  background-color: var(--bt-color-inverse-primary);
			  color: var(--bt-color-inverse-surface);
			}

			/* Bottom row */
			.on-surface {
			  background-color: var(--bt-color-on-surface);
			  color: var(--bt-color-surface);
			}
			.surface-variant {
			  background-color: var(--bt-color-surface-variant);
			  color: var(--bt-color-on-surface-variant);
			}
			.on-surface-variant {
			  background-color: var(--bt-color-on-surface-variant);
			  color: var(--bt-color-surface-variant);
			}
			.outline {
			  background-color: var(--bt-color-outline);
			  color: var(--bt-color-surface);
			}
			.outline-variant {
			  background-color: var(--bt-color-outline-variant);
			  color: var(--bt-color-on-surface);
			}
			.scrim {
			  background-color: var(--bt-color-scrim);
			  color: white;
			}
			.shadow {
			  background-color: var(--bt-color-shadow);
			  color: white;
			}
		</style>
	}
}
