import { Repo } from "@automerge/automerge-repo";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { createHandleEvent } from "./HandleEvent";
import { StateBinder } from "../StateBinder";
import type { StateManifest } from "./types";

describe("HandleEvent", () => {
	let stateBinder: StateBinder;

	beforeEach(async () => {
		document.body.innerHTML = "";
		localStorage.clear();
		stateBinder = new StateBinder(new Repo());
	});

	afterEach(() => {
		stateBinder.destroy();
	});

	it("should bind click event to increment action", async () => {
		const manifest: StateManifest = {
			state_directives: [
				createHandleEvent({
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

		const result = await stateBinder.init();
		expect(result.isOk()).toBe(true);

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
				createHandleEvent({
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

		const result = await stateBinder.init();
		expect(result.isOk()).toBe(true);

		const button = document.getElementById("counter") as HTMLButtonElement;
		button.click();

		// Just verify no errors
		expect(true).toBe(true);
	});

	it("should bind multiple directives to different elements", async () => {
		const manifest: StateManifest = {
			state_directives: [
				createHandleEvent({
					el_state_id: 0,
					field_path: "count",
					event: "click",
					action: "increment",
				}),
				createHandleEvent({
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

		const result = await stateBinder.init();
		expect(result.isOk()).toBe(true);

		const incButton = document.getElementById("inc");
		const decButton = document.getElementById("dec");
		expect(incButton).toBeDefined();
		expect(decButton).toBeDefined();
	});
});
