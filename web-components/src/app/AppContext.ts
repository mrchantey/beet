import { createContext } from "solid-js"

export const defaultAppContext = (props: Partial<AppContext> = {}): AppContext => {
	// Object.assign doesnt work for webcomponent props?
	return {
		src: props.src || "wasm/main.js",
		appName: props.appName || "MyApp",
		canvasId: props.canvasId || "beet-canvas",
		fullHeight: props.fullHeight || false,
	}
}

export type AppContext = {
	appName: string
	canvasId: string
	fullHeight: boolean
	src: string
}
export const AppContext = createContext<AppContext>(defaultAppContext({}))
