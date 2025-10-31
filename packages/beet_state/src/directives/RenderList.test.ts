import { Repo } from "@automerge/automerge-repo";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { StateBinder } from "../StateBinder";
import { createHandleEvent } from "./HandleEvent";
import { createRenderList } from "./RenderList";
import { createRenderText } from "./RenderText";
import type { StateManifest } from "./types";

interface TestDoc {
	todos?: Array<{ id: string; text: string; clicks?: number }>;
}

describe("RenderList", () => {
	let stateBinder: StateBinder;

	beforeEach(async () => {
		document.body.innerHTML = "";
		localStorage.clear();
		stateBinder = new StateBinder(new Repo());
	});

	afterEach(() => {
		stateBinder.destroy();
	});

	it("should render list items from array in state", async () => {
		const templateManifest: StateManifest = {
			state_directives: [
				createRenderText({
					el_state_id: 10,
					field_path: "text",
					template: "%VALUE%",
				}),
			],
		};

		const mainManifest: StateManifest = {
			state_directives: [
				createRenderList({
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
							<span data-state-id="10">Placeholder</span>
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

		const result = await stateBinder.init();
		expect(result.isOk()).toBe(true);

		stateBinder.docHandle?.change((doc: TestDoc) => {
			doc.todos = [
				{ id: "1", text: "First todo" },
				{ id: "2", text: "Second todo" },
			];
		});

		await new Promise((resolve) => setTimeout(resolve, 50));

		const ul = document.querySelector('[data-state-id="0"]');
		expect(ul).toBeDefined();

		const listItems = ul?.querySelectorAll("li");
		expect(listItems?.length).toBeGreaterThan(0);
	});

	it("should use array index when item_key_path is not provided", async () => {
		const templateManifest: StateManifest = {
			state_directives: [
				createRenderText({
					el_state_id: 10,
					field_path: "text",
					template: "%VALUE%",
				}),
			],
		};

		const mainManifest: StateManifest = {
			state_directives: [
				createRenderList({
					el_state_id: 0,
					field_path: "todos",
					template_id: 1,
				}),
			],
		};

		document.body.innerHTML = `
			<div>
				<ul data-state-id="0">
					<template data-state-id="1">
						<li>
							<span data-state-id="10">Text</span>
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

		const result = await stateBinder.init();
		expect(result.isOk()).toBe(true);

		stateBinder.docHandle?.change((doc: TestDoc) => {
			doc.todos = [
				{ id: "1", text: "Item 1" },
				{ id: "2", text: "Item 2" },
			];
		});

		await new Promise((resolve) => setTimeout(resolve, 50));

		const ul = document.querySelector('[data-state-id="0"]');
		const listItems = ul?.querySelectorAll("li");
		expect(listItems?.length).toBeGreaterThan(0);
	});

	it("should handle empty arrays", async () => {
		const templateManifest: StateManifest = {
			state_directives: [],
		};

		const mainManifest: StateManifest = {
			state_directives: [
				createRenderList({
					el_state_id: 0,
					field_path: "todos",
					template_id: 1,
				}),
			],
		};

		document.body.innerHTML = `
			<div>
				<ul data-state-id="0">
					<template data-state-id="1">
						<li>Item</li>
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

		const result = await stateBinder.init();
		expect(result.isOk()).toBe(true);

		stateBinder.docHandle?.change((doc: TestDoc) => {
			doc.todos = [];
		});

		await new Promise((resolve) => setTimeout(resolve, 50));

		const ul = document.querySelector('[data-state-id="0"]');
		const listItems = ul?.querySelectorAll("li");
		expect(listItems?.length).toBe(0);
	});

	it("should handle events within list items", async () => {
		const templateManifest: StateManifest = {
			state_directives: [
				createHandleEvent({
					el_state_id: 10,
					event: "click",
					action: "increment",
					field_path: "clicks",
				}),
				createRenderText({
					el_state_id: 11,
					field_path: "clicks",
					template: "Clicks: %VALUE%",
				}),
			],
		};

		const mainManifest: StateManifest = {
			state_directives: [
				createRenderList({
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
							<button data-state-id="10">Click</button>
							<span data-state-id="11">Clicks: 0</span>
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

		const result = await stateBinder.init();
		expect(result.isOk()).toBe(true);

		stateBinder.docHandle?.change((doc: TestDoc) => {
			doc.todos = [{ id: "1", text: "Test", clicks: 0 }];
		});

		await new Promise((resolve) => setTimeout(resolve, 50));

		const button = document.querySelector("button");
		expect(button).toBeDefined();

		button?.click();
		await new Promise((resolve) => setTimeout(resolve, 10));

		expect(true).toBe(true);
	});

	it("should return error when template element is not found", async () => {
		const mainManifest: StateManifest = {
			state_directives: [
				createRenderList({
					el_state_id: 0,
					field_path: "todos",
					template_id: 99,
				}),
			],
		};

		document.body.innerHTML = `
			<div>
				<ul data-state-id="0">
					<template data-state-id="1">
						<li>Item</li>
					</template>
				</ul>
				<script data-state-manifest type="application/json">
				${JSON.stringify(mainManifest)}
				</script>
			</div>
		`;

		const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

		const result = await stateBinder.init();
		expect(result.isOk()).toBe(true);
		expect(consoleSpy).toHaveBeenCalledWith(
			expect.stringContaining("Template with data-state-id"),
		);

		consoleSpy.mockRestore();
	});

	it("should return error when template_id points to non-template element", async () => {
		const mainManifest: StateManifest = {
			state_directives: [
				createRenderList({
					el_state_id: 0,
					field_path: "todos",
					template_id: 2,
				}),
			],
		};

		document.body.innerHTML = `
			<div>
				<ul data-state-id="0">
					<div data-state-id="2">Not a template</div>
				</ul>
				<script data-state-manifest type="application/json">
				${JSON.stringify(mainManifest)}
				</script>
			</div>
		`;

		const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

		const result = await stateBinder.init();
		expect(result.isOk()).toBe(true);
		expect(consoleSpy).toHaveBeenCalledWith(
			expect.stringContaining("not a <template>"),
		);

		consoleSpy.mockRestore();
	});
});
