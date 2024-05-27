import { useParams } from '@solidjs/router'
import { AppContext, defaultAppContext } from '../AppContext'
import { beetExamples } from '../examples'
import Runner from './Runner'


type ExampleParams = {
	name: string
}


export const ExampleRunner = () => {
	let params = useParams<ExampleParams>()

	let example = beetExamples[params.name]

	if (example === undefined) {
		return <div>Example not found: {params.name}</div>
	}

	let ctx = defaultAppContext(example)

	return (
		<AppContext.Provider value={ctx}>
			<Runner />
		</AppContext.Provider>
	)
}

export default ExampleRunner