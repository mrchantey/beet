import { useNavigate } from '@solidjs/router'
import { List, ListItem, ListItemButton, ListItemText, Stack, Typography } from '@suid/material'
import { For } from 'solid-js'
import { beetExamples } from '../examples'

export const Home = () => {

	let items = Object.entries(beetExamples)
	let navigate = useNavigate()

	return (
		<Stack sx={{ padding: '3em' }}>
			<Typography variant="h3">Beet Demos</Typography>
			<Typography variant="body1">
				Welcome to the Beet demo app. This open source project is for helping you share your Beet apps. Select an example or use your own
				app by specifiying the query parameter like so:
				<br />
				<br />
				<code>https://demo.beetmash.com/run?src=http://example.com/wasm/main.js</code>
			</Typography>
			<nav>
				<List>
					<For each={items} >
						{([key, app], index) =>
							<ListItem data-index={index()}>
								<ListItemButton onClick={() => navigate(`/examples/${key}`)}>
									<ListItemText primary={app.appName} />
								</ListItemButton>
							</ListItem>
						}
					</For>
				</List>
			</nav>
		</Stack>
	)
}
export default Home