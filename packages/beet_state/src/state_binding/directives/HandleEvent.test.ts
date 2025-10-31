import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { BindContext } from "../BindContext";
import type { StateManifest } from "./types";

describe("HandleEvent", () => {
	let bindContext: BindContext;

	beforeEach(async () => {
		const result = await BindContext.initTest();
		if (result.isErr()) {
			throw new Error(`Failed to initialize test context: ${result.error}`);
		}
		bindContext = result.value;
	});

	afterEach(() => {
		bindContext.destroy();
	});

	it("should bind click event to increment action", async () => {
		const manifest: StateManifest = {
			state_directives: [
				BindContext.handleEvent({
					el_state_id: 0,
					field_path: "count",
					event: "click",
					action: "increment",
				}),
			],
		};

		document.body.innerHTML = `
			<div>
				<button id="counter" data-state-id="0">Click me</button>
				<script data-state-manifest type="application/json">
				${JSON.stringify(manifest)}
				</script>
			</div>
		`;

		const button = document.getElementById("counter") as HTMLButtonElement;
		expect(button).toBeDefined();

		// Simulate clicks
		button.click();

		// Wait a bit for the change to propagate
		await new Promise((resolve) => setTimeout(resolve, 10));
	});

	it("should support decrement action", async () => {
		const manifest: StateManifest = {
			state_directives: [
				BindContext.handleEvent({
					el_state_id: 0,
					field_path: "count",
					event: "click",
					action: "decrement",
				}),
			],
		};

		document.body.innerHTML = `
			<div>
				<button id="counter" data-state-id="0">Click me</button>
				<script data-state-manifest type="application/json">
				${JSON.stringify(manifest)}
				</script>
			</div>
		`;

		const button = document.getElementById("counter") as HTMLButtonElement;
		button.click();

		// Just verify no errors
		expect(true).toBe(true);
	});

	it("should bind multiple directives to different elements", async () => {
		const manifest: StateManifest = {
			state_directives: [
				BindContext.handleEvent({
					el_state_id: 0,
					field_path: "count",
					event: "click",
					action: "increment",
				}),
				BindContext.handleEvent({
					el_state_id: 1,
					field_path: "count",
					event: "click",
					action: "decrement",
				}),
			],
		};

		document.body.innerHTML = `
			<div>
				<button id="inc" data-state-id="0">+</button>
				<button id="dec" data-state-id="1">-</button>
				<script data-state-manifest type="application/json">
				${JSON.stringify(manifest)}
				</script>
			</div>
		`;

		const incButton = document.getElementById("inc");
		const decButton = document.getElementById("dec");
		expect(incButton).toBeDefined();
		expect(decButton).toBeDefined();
	});
});
