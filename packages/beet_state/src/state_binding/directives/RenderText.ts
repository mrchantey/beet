import { ok, type Result } from "neverthrow";
import { BindContext } from "../BindContext";
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
	const set = () => {
		const value = context.getValueByPath(config.field_path);
		const template = config.template;
		const text = template.replace("%VALUE%", String(value));
		element.textContent = text;
	};

	// set once on init and every time on change
	set();
	context.onChange(set);

	return ok({});
}
