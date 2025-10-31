import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { BindContext } from "../BindContext";
import type { StateManifest } from "./types";

describe("RenderText", () => {
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

	it("should set up RenderText binding without errors", async () => {
		const manifest: StateManifest = {
			state_directives: [
				BindContext.renderText({
					el_state_id: 0,
					field_path: "count",
					template: "The value is %VALUE%",
				}),
			],
		};

		document.body.innerHTML = `
			<div>
				<p id="display" data-state-id="0">Initial</p>
				<script data-state-manifest type="application/json">
				${JSON.stringify(manifest)}
				</script>
			</div>
		`;

		// Wait for MutationObserver to process
		await new Promise((resolve) => setTimeout(resolve, 50));

		const display = document.getElementById("display");
		expect(display).toBeDefined();

		// Verify binding was created
		const disposers = (bindContext as any).disposers;
		expect(disposers.length).toBeGreaterThan(0);
	});

	it("should support multiple RenderText directives", async () => {
		const manifest: StateManifest = {
			state_directives: [
				BindContext.renderText({
					el_state_id: 0,
					field_path: "count",
					template: "Count: %VALUE%",
				}),
				BindContext.renderText({
					el_state_id: 1,
					field_path: "name",
					template: "Name: %VALUE%",
				}),
			],
		};

		document.body.innerHTML = `
			<div>
				<p id="count" data-state-id="0">Count</p>
				<p id="name" data-state-id="1">Name</p>
				<script data-state-manifest type="application/json">
				${JSON.stringify(manifest)}
				</script>
			</div>
		`;

		// Wait for MutationObserver to process
		await new Promise((resolve) => setTimeout(resolve, 50));

		const countDisplay = document.getElementById("count");
		const nameDisplay = document.getElementById("name");
		expect(countDisplay).toBeDefined();
		expect(nameDisplay).toBeDefined();

		// Verify both bindings were created
		const disposers = (bindContext as any).disposers;
		expect(disposers.length).toBe(2);
	});
});
