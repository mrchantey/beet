
import SendIcon from "@suid/icons-material/Send"
import { Button, Stack, TextField } from '@suid/material'
import { createSignal } from 'solid-js'
import { sendPlayerMessage } from '../beet/message'


export const MessageBox = () => {

	const [text, setText] = createSignal('')

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