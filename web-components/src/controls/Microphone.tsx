import { FormControlLabel, Stack, Switch } from "@suid/material"
import styles from './Controls.module.css'
import { PhraseDetector } from "./phraseDetector"

export const Microphone = () => {

	let detector = new PhraseDetector()

	let handleChange = (_: any, checked: boolean) => {
		if (checked)
			detector.start()
		else
			detector.stop()
	}

	return (
		<Stack>
			<FormControlLabel
				onChange={handleChange}
				class={styles.microphone}
				labelPlacement="start"
				control={<Switch />} label="Microphone" />
		</Stack>
	)
}