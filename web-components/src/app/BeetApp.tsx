// import roboto300 from '@fontsource/roboto/300.css?inline'
// import roboto400 from '@fontsource/roboto/400.css?inline'
// import roboto500 from '@fontsource/roboto/500.css?inline'
// import roboto700 from '@fontsource/roboto/700.css?inline'

import { customElement, noShadowDOM } from "solid-element"
import { Component } from "solid-js"
import { App } from './App'
import { AppContext, defaultAppContext } from "./AppContext"
// import inlineStyles from './BeetApp.module.css?inline'

const BeetAppElement: Component = (props: Partial<AppContext>) => {
	// console.log("BeetAppElement", props.fullHeight, props.src)

	noShadowDOM()
	return (
		<>
			{/* <style>{roboto300}</style>
			<style>{roboto400}</style>
			<style>{roboto500}</style>
			<style>{roboto700}</style> */}
			{/* <style>{inlineStyles}</style> */}
			<App {...props} />
		</>
	)
}

customElement("beet-app", defaultAppContext(), BeetAppElement)
