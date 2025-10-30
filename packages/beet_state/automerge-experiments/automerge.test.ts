import { Repo } from "@automerge/automerge-repo";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

interface TestDoc {
	count: number;
	name?: string;
	items?: string[];
}

describe("Automerge Repo", () => {
	let repo: Repo;

	beforeEach(() => {
		// Create a new repo instance for each test
		repo = new Repo();
	});

	afterEach(() => {
		// Clean up
		repo.networkSubsystem.adapters.forEach((adapter) => {
			adapter?.disconnect?.();
		});
	});

	it("should create a new document", () => {
		const handle = repo.create<TestDoc>();
		expect(handle).toBeDefined();
		expect(handle.documentId).toBeDefined();
		expect(typeof handle.documentId).toBe("string");
	});

	it("should initialize document with data", () => {
		const handle = repo.create<TestDoc>();
		handle.change((doc) => {
			doc.count = 0;
			doc.name = "Test Document";
		});

		const doc = handle.doc();
		expect(doc?.count).toBe(0);
		expect(doc?.name).toBe("Test Document");
	});
	it("should update document values", () => {
		const handle = repo.create<TestDoc>();

		handle.change((doc) => {
			doc.count = 5;
		});

		expect(handle.doc()?.count).toBe(5);

		handle.change((doc) => {
			doc.count = 10;
		});

		expect(handle.doc()?.count).toBe(10);
	});

	it("should increment counter", () => {
		const handle = repo.create<TestDoc>();

		handle.change((doc) => {
			doc.count = 0;
		});

		handle.change((doc) => {
			doc.count += 1;
		});

		handle.change((doc) => {
			doc.count += 1;
		});

		expect(handle.doc()?.count).toBe(2);
	});

	it("should handle arrays", () => {
		const handle = repo.create<TestDoc>();

		handle.change((doc) => {
			doc.items = [];
		});

		handle.change((doc) => {
			doc.items?.push("first");
			doc.items?.push("second");
		});

		const doc = handle.doc();
		expect(doc?.items).toHaveLength(2);
		expect(doc?.items).toEqual(["first", "second"]);
	});
	it("should find document by ID", async () => {
		const handle1 = repo.create<TestDoc>();
		handle1.change((doc) => {
			doc.count = 42;
		});

		const docId = handle1.documentId;
		const handle2 = await repo.find<TestDoc>(docId);

		expect(handle2.documentId).toBe(docId);
		expect(handle2.doc()?.count).toBe(42);
	});

	it("should handle multiple documents", () => {
		const handle1 = repo.create<TestDoc>();
		const handle2 = repo.create<TestDoc>();

		handle1.change((doc) => {
			doc.count = 1;
			doc.name = "First";
		});

		handle2.change((doc) => {
			doc.count = 2;
			doc.name = "Second";
		});

		expect(handle1.doc()?.name).toBe("First");
		expect(handle2.doc()?.name).toBe("Second");
		expect(handle1.documentId).not.toBe(handle2.documentId);
	});

	it("should preserve document state across changes", () => {
		const handle = repo.create<TestDoc>();

		handle.change((doc) => {
			doc.count = 5;
			doc.name = "Persistent";
		});

		handle.change((doc) => {
			doc.count += 1;
		});

		const doc = handle.doc();
		expect(doc?.count).toBe(6);
		expect(doc?.name).toBe("Persistent");
	});

	it("should handle undefined initial values", () => {
		const handle = repo.create<TestDoc>();
		const doc = handle.doc();

		expect(doc?.count).toBeUndefined();
		expect(doc?.name).toBeUndefined();
		expect(doc?.items).toBeUndefined();
	});
});
