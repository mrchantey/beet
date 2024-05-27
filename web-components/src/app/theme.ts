import { createPalette, createTheme } from "@suid/material"
import { purple } from "@suid/material/colors"
import { createMemo, createSignal } from "solid-js"


type Mode = "light" | "dark"

export const useBeetTheme = () => {

	let mql = window.matchMedia("(prefers-color-scheme: dark)")

	const [mode, setMode] = createSignal<Mode>(mql.matches ? "dark" : "light")

	mql.addEventListener("change", () => {
		setMode(mql.matches ? "dark" : "light")
	})

	const palette = createMemo(() => {
		return createPalette({
			mode: mode(),
			primary: {
				main: purple[500],
			},
			secondary: {
				// This is green.A700 as hex.
				main: "#11cb5f",
			},
		})
	})

	const theme = createTheme({ palette: palette })

	return theme
}
