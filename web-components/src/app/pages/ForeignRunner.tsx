import { AppContext, defaultAppContext } from '../AppContext'
import Runner from './Runner'


export const ForeignRunner = () => {

	let ctx = defaultAppContext()

	return (
		<AppContext.Provider value={ctx}>
			<Runner />
		</AppContext.Provider>
	)
}

export default ForeignRunner