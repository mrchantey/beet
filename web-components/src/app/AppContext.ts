import { createContext } from "solid-js"
import { beetExamples } from "./examples"


export const getQueryParams = (): Partial<AppContext> => {
	const params = new URLSearchParams(window.location.search)
	let queryParams: any = {}
	for (let param of params) {
		let [key, value]: any = param
		if (value === 'true') value = true
		if (value === 'false') value = false
		queryParams[key] = value
	}
	return queryParams
}


export const defaultAppContext = (props: Partial<AppContext> = {}): AppContext => {
	// Object.assign doesnt work for webcomponent props?

	let obj = getQueryParams()
	if (obj.example) {
		Object.assign(obj, beetExamples[obj.example])
	}

	return Object.assign({
		src: props.src || "wasm/main.js",
		appName: props.appName || "MyApp",
		canvasId: props.canvasId || "beet-canvas",
		initialPrompt: props.initialPrompt || '',
		fullHeight: props.fullHeight || true,
		sidePanel: props.sidePanel || false,
		startButton: props.startButton || false,
		loadEvent: props.loadEvent || false
	}, obj)
}

export type AppContext = {
	appName: string
	canvasId: string
	fullHeight: boolean
	initialPrompt: string
	startButton: boolean
	sidePanel: boolean
	loadEvent: boolean
	src: string
	example?: string
}
export const AppContext = createContext<AppContext>(defaultAppContext({}))
