import { createTheme } from "@suid/material"
import { purple } from "@suid/material/colors"

export const customTheme = createTheme({
	palette: {
		mode: "dark",
		// primary: {
		// 	main: "#1976d2",
		// 	// contrastText: "white",
		// },
		primary: {
			// Purple and green play nicely together.
			main: purple[500],
		},
		secondary: {
			// This is green.A700 as hex.
			main: "#11cb5f",
		},
	},
})
