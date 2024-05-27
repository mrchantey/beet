
import SendIcon from "@suid/icons-material/Send"
import { Button, Stack, TextField } from '@suid/material'
import { createSignal, useContext } from 'solid-js'
import { AppContext } from "../app/AppContext"
import { sendPlayerMessage } from '../beet/message'


export const MessageBox = () => {

	let ctx = useContext(AppContext)

	const [text, setText] = createSignal(ctx.initialPrompt)

	let submit = () => {
		sendPlayerMessage(text())
		setText('')
	}

	return (
		<Stack direction="row">
			<TextField
				fullWidth
				value={text()}
				onChange={(e) => setText(e.currentTarget.value)}
				onKeyUp={(e) => { if (e.key === 'Enter') submit() }}
				label="Send a message"
				variant="outlined"
				size="small"
			/>
			<Button aria-label="send" variant="contained" onClick={submit} endIcon={<SendIcon />}>
			</Button>
		</Stack>
	)
}