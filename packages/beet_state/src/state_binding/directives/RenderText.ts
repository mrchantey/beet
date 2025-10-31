import { createRoot, createEffect } from "solid-js";
import { makeDocumentProjection } from "@automerge/automerge-repo-solid-primitives";
import { ok, type Result } from "neverthrow";
import type { BindContext } from "../BindContext";
import type { BindElement, BindResult, FieldLocation } from "./types";

/**
 * Defines how a DOM element's text should update when a document field changes.
 */
export type RenderText = BindElement &
	FieldLocation & {
		/** Discriminant for union type */
		kind: "render_text";

		/** Template string with %VALUE% placeholder for the field value */
		template: string;
	};

/**
 * Bind a RenderText directive to an element
 */
export function bindRenderText(
	element: Element,
	config: RenderText,
	context: BindContext,
): Result<BindResult, string> {
	const { docHandle } = context;

	// Initialize the field if it doesn't exist
	const fieldPath = config.field_path;
	const currentDoc = docHandle!.doc();
	if (
		currentDoc &&
		context.getValueByPath(currentDoc, fieldPath) === undefined
	) {
		docHandle!.change((doc: any) => {
			if (context.getValueByPath(doc, fieldPath) === undefined) {
				context.setValueByPath(doc, fieldPath, 0);
			}
		});
	}

	const docProjection = makeDocumentProjection(docHandle!);

	const dispose = createRoot((dispose) => {
		createEffect(() => {
			const value =
				context.getValueByPath(docProjection as any, fieldPath) ?? 0;
			const template = config.template;
			const text = template.replace("%VALUE%", String(value));
			element.textContent = text;
		});
		return dispose;
	});

	return ok({ dispose });
}
