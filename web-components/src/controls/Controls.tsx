import { Stack, Typography } from "@suid/material"
import { useContext } from "solid-js"
import { AppContext } from "../app/AppContext"
import { MessageBox } from "./MessageBox"
import { Microphone } from "./Microphone"

import controls from './Controls.module.css'

export const Controls = () => {

	let appContext = useContext(AppContext)

	return (
		<Stack class={controls.controls} direction="column" spacing={1}>
			<Typography variant="h5">{appContext.appName}</Typography>
			<MessageBox />
			<Microphone />
		</Stack>
	)
}