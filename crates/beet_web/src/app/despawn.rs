use crate::prelude::*;
use bevy::prelude::*;





pub fn despawn_elements(
	mut map: NonSendMut<DomSimElements>,
	mut removed: RemovedComponents<DomSimEntity>,
) {
	for removed in removed.read() {
		if let Some(el) = map.0.remove(&removed) {
			el.remove();
		}
	}
}
