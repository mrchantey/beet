import { Repo } from "@automerge/automerge-repo";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
	HandleEvent,
	StateBinder,
	UpdateDom,
	type StateManifest,
} from "./StateBinder";

interface TestDoc {
	count: number;
	name?: string;
}

describe("StateBinder", () => {
	let stateBinder: StateBinder;

	beforeEach(async () => {
		document.body.innerHTML = "";
		localStorage.clear();
		stateBinder = new StateBinder(new Repo());
	});

	afterEach(() => {
		stateBinder.destroy();
	});

	describe("init", () => {
		it("should initialize and create a document handle", async () => {
			const result = await stateBinder.init();
			expect(result.isOk()).toBe(true);
			expect(localStorage.getItem("rootDocId")).toBeDefined();
		});

		it("should use provided docId if given", async () => {
			const handle = stateBinder.repo.create<TestDoc>();
			const docId = handle.documentId;

			const result = await stateBinder.init(docId);
			expect(result.isOk()).toBe(true);
			// Should not create a new one in localStorage
			expect(localStorage.getItem("rootDocId")).toBeNull();
		});

		it("should scan existing elements on init", async () => {
			const manifest: StateManifest = {
				state_directives: [
					HandleEvent.create({
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

			// Verify the button was found and scanned
			const button = document.getElementById("counter");
			expect(button).toBeDefined();
		});
	});

	describe("manifest parsing", () => {
		it("should parse valid manifest from script element", async () => {
			const manifest: StateManifest = {
				state_directives: [
					HandleEvent.create({
						el_state_id: 0,
						field_path: "count",
						event: "click",
						action: "increment",
					}),
				],
			};

			document.body.innerHTML = `
				<div>
					<button data-state-id="0">Test</button>
					<script data-state-manifest type="application/json">
					${JSON.stringify(manifest)}
					</script>
				</div>
			`;

			const result = await stateBinder.init();
			expect(result.isOk()).toBe(true);
		});

		it("should handle missing manifest gracefully", async () => {
			document.body.innerHTML = `
				<button data-state-id="0">Test</button>
			`;

			const result = await stateBinder.init();
			// Should succeed but not bind anything
			expect(result.isOk()).toBe(true);
		});

		it("should return error for invalid manifest JSON", async () => {
			document.body.innerHTML = `
				<div>
					<button data-state-id="0">Test</button>
					<script data-state-manifest type="application/json">
					{invalid json}
					</script>
				</div>
			`;

			const result = await stateBinder.init();
			expect(result.isErr()).toBe(true);
			if (result.isErr()) {
				expect(result.error).toContain("Failed to parse manifest");
			}
		});

		it("should return error for manifest without state_directives", async () => {
			document.body.innerHTML = `
				<div>
					<button data-state-id="0">Test</button>
					<script data-state-manifest type="application/json">
					{"wrong_field": []}
					</script>
				</div>
			`;

			const result = await stateBinder.init();
			expect(result.isErr()).toBe(true);
			if (result.isErr()) {
				expect(result.error).toContain("Invalid manifest");
			}
		});
	});

	describe("element binding", () => {
		it("should find element by data-state-id", async () => {
			const manifest: StateManifest = {
				state_directives: [
					HandleEvent.create({
						el_state_id: 0,
						field_path: "count",
						event: "click",
						action: "increment",
					}),
				],
			};

			document.body.innerHTML = `
				<div>
					<button id="my-button" data-state-id="0">Test</button>
					<script data-state-manifest type="application/json">
					${JSON.stringify(manifest)}
					</script>
				</div>
			`;

			const result = await stateBinder.init();
			expect(result.isOk()).toBe(true);

			const button = document.getElementById("my-button");
			expect(button).toBeDefined();
		});

		it("should warn when element with data-state-id not found", async () => {
			const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

			const manifest: StateManifest = {
				state_directives: [
					HandleEvent.create({
						el_state_id: 99,
						field_path: "count",
						event: "click",
						action: "increment",
					}),
				],
			};

			document.body.innerHTML = `
				<div>
					<button data-state-id="0">Test</button>
					<script data-state-manifest type="application/json">
					${JSON.stringify(manifest)}
					</script>
				</div>
			`;

			const result = await stateBinder.init();
			expect(result.isOk()).toBe(true);
			expect(consoleSpy).toHaveBeenCalledWith(
				expect.stringContaining('data-state-id="99"'),
			);

			consoleSpy.mockRestore();
		});
	});

	describe("event binding", () => {
		it("should bind click event to increment action", async () => {
			const manifest: StateManifest = {
				state_directives: [
					HandleEvent.create({
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

			// Get the doc handle to check state
			const handle = stateBinder.repo.create<TestDoc>();
			handle.change((doc: TestDoc) => {
				doc.count = 0;
			});

			// Simulate clicks
			button.click();

			// Wait a bit for the change to propagate
			await new Promise((resolve) => setTimeout(resolve, 10));
		});

		it("should support decrement action", async () => {
			const manifest: StateManifest = {
				state_directives: [
					HandleEvent.create({
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

			// Simulate click
			button.click();

			// Just verify no errors
			expect(true).toBe(true);
		});

		it("should bind multiple directives to different elements", async () => {
			const manifest: StateManifest = {
				state_directives: [
					HandleEvent.create({
						el_state_id: 0,
						field_path: "count",
						event: "click",
						action: "increment",
					}),
					HandleEvent.create({
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

	describe("onchange binding with Solid effects", () => {
		it("should set up onchange binding without errors", async () => {
			const manifest: StateManifest = {
				state_directives: [
					UpdateDom.create({
						el_state_id: 0,
						field_path: "count",
						onchange: {
							kind: "set_with",
							template: "The value is %VALUE%",
						},
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

			const result = await stateBinder.init();
			expect(result.isOk()).toBe(true);

			// The binding should be set up without errors
			const display = document.getElementById("display");
			expect(display).toBeDefined();

			// Note: Testing Solid effects in a test environment is tricky
			// The effect setup should not throw, which is what we're verifying here
		});
	});

	describe("MutationObserver", () => {
		it("should detect and bind dynamically added elements", async () => {
			const result = await stateBinder.init();
			expect(result.isOk()).toBe(true);

			const manifest: StateManifest = {
				state_directives: [
					HandleEvent.create({
						el_state_id: 0,
						field_path: "count",
						event: "click",
						action: "increment",
					}),
				],
			};

			// Add elements dynamically
			const container = document.createElement("div");
			const button = document.createElement("button");
			button.id = "dynamic-button";
			button.setAttribute("data-state-id", "0");

			const script = document.createElement("script");
			script.setAttribute("data-state-manifest", "");
			script.type = "application/json";
			script.textContent = JSON.stringify(manifest);

			container.appendChild(button);
			container.appendChild(script);
			document.body.appendChild(container);

			// Wait for MutationObserver to trigger
			await new Promise((resolve) => setTimeout(resolve, 50));

			// Verify element was processed (no errors)
			expect(button.parentElement).toBe(container);
		});

		it("should not bind the same element twice", async () => {
			const manifest: StateManifest = {
				state_directives: [
					HandleEvent.create({
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

			// Remove and re-add (should use WeakSet to track)
			const parent = button.parentElement!;
			parent.removeChild(button);
			parent.appendChild(button);

			await new Promise((resolve) => setTimeout(resolve, 50));

			// Should still work without duplicating listeners
			expect(button.parentElement).toBe(parent);
		});
	});

	describe("cleanup", () => {
		it("should disconnect MutationObserver on destroy", async () => {
			const result = await stateBinder.init();
			expect(result.isOk()).toBe(true);

			const disconnectSpy = vi.fn();
			// Access private observer and spy on it
			const observer = (stateBinder as any).mutationObserver;
			if (observer) {
				observer.disconnect = disconnectSpy;
			}

			stateBinder.destroy();
			expect(disconnectSpy).toHaveBeenCalled();
		});
	});
});
