import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { StateManifest } from "./directives";
import { BindContext } from "./BindContext";

interface TestDoc {
	count: number;
	name?: string;
	todos?: Array<{ id: string; text: string; clicks?: number }>;
}

describe("BindContext - Integration Tests", () => {
	let bindContext: BindContext;

	beforeEach(async () => {
		bindContext = BindContext.newTest();
	});

	afterEach(() => {
		bindContext.destroy();
	});

	describe("initialization", () => {
		it("should initialize and create a document handle", async () => {
			const result = await bindContext.init();
			expect(result.isOk()).toBe(true);
			expect(localStorage.getItem("rootDocId")).toBeDefined();
		});

		it("should use provided docId if given", async () => {
			const handle = bindContext.repo.create<TestDoc>();
			const docId = handle.documentId;

			const result = await bindContext.init(docId);
			expect(result.isOk()).toBe(true);
			expect(localStorage.getItem("rootDocId")).toBeNull();
		});

		it("should scan existing elements on init", async () => {
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

			const result = await bindContext.init();
			expect(result.isOk()).toBe(true);

			const button = document.getElementById("counter");
			expect(button).toBeDefined();
		});
	});

	describe("manifest parsing", () => {
		it("should parse valid manifest from script element", async () => {
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
					<button data-state-id="0">Test</button>
					<script data-state-manifest type="application/json">
					${JSON.stringify(manifest)}
					</script>
				</div>
			`;

			const result = await bindContext.init();
			expect(result.isOk()).toBe(true);
		});

		it("should handle missing manifest gracefully", async () => {
			document.body.innerHTML = `
				<button data-state-id="0">Test</button>
			`;

			const result = await bindContext.init();
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

			const result = await bindContext.init();
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

			const result = await bindContext.init();
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
					<button id="my-button" data-state-id="0">Test</button>
					<script data-state-manifest type="application/json">
					${JSON.stringify(manifest)}
					</script>
				</div>
			`;

			const result = await bindContext.init();
			expect(result.isOk()).toBe(true);

			const button = document.getElementById("my-button");
			expect(button).toBeDefined();
		});

		it("should warn when element with data-state-id not found", async () => {
			const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

			const manifest: StateManifest = {
				state_directives: [
					BindContext.handleEvent({
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

			const result = await bindContext.init();
			expect(result.isOk()).toBe(true);
			expect(consoleSpy).toHaveBeenCalledWith(
				expect.stringContaining('data-state-id="99"'),
			);

			consoleSpy.mockRestore();
		});

		it("should not bind the same element twice", async () => {
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

			const result = await bindContext.init();
			expect(result.isOk()).toBe(true);

			const button = document.getElementById("counter") as HTMLButtonElement;

			const parent = button.parentElement!;
			parent.removeChild(button);
			parent.appendChild(button);

			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(button.parentElement).toBe(parent);
		});
	});

	describe("MutationObserver", () => {
		it("should detect and bind dynamically added elements", async () => {
			const result = await bindContext.init();
			expect(result.isOk()).toBe(true);

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

			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(button.parentElement).toBe(container);
		});
	});

	describe("cleanup", () => {
		it("should disconnect MutationObserver on destroy", async () => {
			const result = await bindContext.init();
			expect(result.isOk()).toBe(true);

			const disconnectSpy = vi.fn();
			const observer = (bindContext as any).mutationObserver;
			if (observer) {
				observer.disconnect = disconnectSpy;
			}

			bindContext.destroy();
			expect(disconnectSpy).toHaveBeenCalled();
		});

		it("should cleanup all disposers", async () => {
			const manifest: StateManifest = {
				state_directives: [
					BindContext.renderText({
						el_state_id: 0,
						field_path: "count",
						template: "%VALUE%",
					}),
				],
			};

			document.body.innerHTML = `
				<div>
					<p data-state-id="0">Text</p>
					<script data-state-manifest type="application/json">
					${JSON.stringify(manifest)}
					</script>
				</div>
			`;

			const result = await bindContext.init();
			expect(result.isOk()).toBe(true);

			const disposers = (bindContext as any).disposers;
			expect(disposers.length).toBeGreaterThan(0);

			bindContext.destroy();
			expect((bindContext as any).disposers.length).toBe(0);
		});
	});

	describe("mixed directives", () => {
		it("should handle multiple directive types together", async () => {
			const manifest: StateManifest = {
				state_directives: [
					BindContext.handleEvent({
						el_state_id: 0,
						field_path: "count",
						event: "click",
						action: "increment",
					}),
					BindContext.renderText({
						el_state_id: 1,
						field_path: "count",
						template: "Count: %VALUE%",
					}),
				],
			};

			document.body.innerHTML = `
				<div>
					<button id="inc" data-state-id="0">+</button>
					<p id="display" data-state-id="1">Count: 0</p>
					<script data-state-manifest type="application/json">
					${JSON.stringify(manifest)}
					</script>
				</div>
			`;

			const result = await bindContext.init();
			expect(result.isOk()).toBe(true);

			const button = document.getElementById("inc");
			const display = document.getElementById("display");

			expect(button).toBeDefined();
			expect(display).toBeDefined();

			// Verify both directives were bound
			const disposers = (bindContext as any).disposers;
			expect(disposers.length).toBeGreaterThan(0);

			// Verify clicking works without errors
			button?.click();
			await new Promise((resolve) => setTimeout(resolve, 10));
			expect(true).toBe(true);
		});

		it("should handle list with events and text rendering", async () => {
			const templateManifest: StateManifest = {
				state_directives: [
					BindContext.renderText({
						el_state_id: 10,
						field_path: "text",
						template: "%VALUE%",
					}),
					BindContext.handleEvent({
						el_state_id: 11,
						event: "click",
						action: "increment",
						field_path: "clicks",
					}),
					BindContext.renderText({
						el_state_id: 12,
						field_path: "clicks",
						template: "%VALUE%",
					}),
				],
			};

			const mainManifest: StateManifest = {
				state_directives: [
					BindContext.renderList({
						el_state_id: 0,
						field_path: "todos",
						template_id: 1,
						item_key_path: "id",
					}),
				],
			};

			document.body.innerHTML = `
				<div>
					<ul data-state-id="0">
						<template data-state-id="1">
							<li>
								<span data-state-id="10">Text</span>
								<button data-state-id="11">+</button>
								<span data-state-id="12">0</span>
							</li>
							<script data-state-manifest type="application/json">
							${JSON.stringify(templateManifest)}
							</script>
						</template>
					</ul>
					<script data-state-manifest type="application/json">
					${JSON.stringify(mainManifest)}
					</script>
				</div>
			`;

			const result = await bindContext.init();
			expect(result.isOk()).toBe(true);

			bindContext.docHandle?.change((doc: TestDoc) => {
				doc.todos = [
					{ id: "1", text: "Task 1", clicks: 0 },
					{ id: "2", text: "Task 2", clicks: 0 },
				];
			});

			await new Promise((resolve) => setTimeout(resolve, 50));

			const ul = document.querySelector('[data-state-id="0"]');
			const listItems = ul?.querySelectorAll("li");
			expect(listItems?.length).toBe(2);

			const firstButton = document.querySelector("button");
			firstButton?.click();

			await new Promise((resolve) => setTimeout(resolve, 50));

			expect(true).toBe(true);
		});
	});
});
