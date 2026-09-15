//! [`PreBoot`]: what a served page runs before its wasm arrives, and what
//! the world does with it once it has.
use beet_core::prelude::*;

/// The pieces of a served page that run before the world exists, and their
/// counterparts once it does.
///
/// A classic inline script in the head, rendered by `<Wasm>` through
/// [`script`](Self::script), does only what must happen before the first
/// paint:
///
/// - it installs one capture listener queueing every `click` and `submit`
///   (the latter prevented, so a form never navigates away from the page
///   about to serve it) under `globalThis.beetPreBoot`, replayed against the
///   bound nodes once the world has adopted the page
///   ([`take_queue`](Self::take_queue)); a typed value needs no replay,
///   adoption reads it;
/// - it hides the body for a returning editor: a browser holding a fork of
///   the page's repo says so under one local-storage bit
///   ([`StoreUri::fork_mark`], written by the fork on its first write), for
///   whom the served page, the published one, is wrong; the first paint
///   reveals it ([`reveal`](Self::reveal));
/// - it decides when the wasm loads ([`BootPolicy`]): at once, or on the
///   first click or toggle of an element marked [`INTENT`](Self::INTENT), a
///   returning editor booting at once either way.
///
/// All store logic stays in the world: the script reads one bit and queues
/// events, nothing more.
pub struct PreBoot;

impl PreBoot {
	/// The global the queue lives under until the world takes it.
	pub const GLOBAL: &'static str = "beetPreBoot";
	/// The id of the style hiding the body for a returning editor.
	pub const STYLE_ID: &'static str = "beet-pre-boot";
	/// The attribute marking an element whose click or toggle is the intent
	/// to boot: reaching for edit mode loads the wasm.
	pub const INTENT: &'static str = "data-beet-boot";

	/// The inline script: the capture queue always; the pre-paint bit when
	/// the page names a repo (`fork_mark`, see [`StoreUri::fork_mark`]); and,
	/// booting on intent, the module `loader` body to inject on the first
	/// marked click or toggle, at once for a returning editor.
	pub fn script(fork_mark: Option<&str>, loader: Option<&str>) -> String {
		let global = Self::GLOBAL;
		let mut script = format!(
			"const q=[],c=e=>q.push(e),s=e=>{{e.preventDefault();q.push(e)}};\
			addEventListener(\"click\",c,true);addEventListener(\"submit\",s,true);\
			globalThis.{global}={{q,off(){{removeEventListener(\"click\",c,true);removeEventListener(\"submit\",s,true)}}}};\
			let f=false;"
		);
		if let Some(mark) = fork_mark {
			let style_id = Self::STYLE_ID;
			script.push_str(&format!(
				"try{{f=!!localStorage.getItem({mark:?})}}catch{{}}\
				if(f){{const st=document.createElement(\"style\");st.id={style_id:?};\
				st.textContent=\"body{{visibility:hidden}}\";document.head.appendChild(st)}}"
			));
		}
		if let Some(loader) = loader {
			let intent = Self::INTENT;
			script.push_str(&format!(
				"const b=()=>{{removeEventListener(\"click\",i,true);removeEventListener(\"toggle\",i,true);\
				const m=document.createElement(\"script\");m.type=\"module\";m.textContent={loader:?};document.head.appendChild(m)}},\
				i=e=>{{e.target?.closest?.(\"[{intent}]\")&&b()}};\
				if(f)b();else{{addEventListener(\"click\",i,true);addEventListener(\"toggle\",i,true)}}"
			));
		}
		format!("(()=>{{{script}}})()")
	}

	/// The events the page queued before the world existed, its capture
	/// listeners removed and the global gone: the world's own listeners take
	/// over from here, so nothing is delivered twice.
	#[cfg(target_arch = "wasm32")]
	pub fn take_queue() -> Vec<web_sys::Event> {
		use wasm_bindgen::JsCast;
		use wasm_bindgen::JsValue;
		let global = js_sys::global();
		let key = JsValue::from_str(Self::GLOBAL);
		let Some(pre_boot) = js_sys::Reflect::get(&global, &key)
			.ok()
			.filter(|value| value.is_object())
		else {
			return Vec::new();
		};
		if let Ok(off) = js_sys::Reflect::get(&pre_boot, &"off".into())
			&& let Some(off) = off.dyn_ref::<js_sys::Function>()
		{
			off.call0(&pre_boot).ok();
		}
		let queue = js_sys::Reflect::get(&pre_boot, &"q".into())
			.map(|queue| js_sys::Array::from(&queue))
			.unwrap_or_default();
		js_sys::Reflect::delete_property(
			global.unchecked_ref::<js_sys::Object>(),
			&key,
		)
		.ok();
		queue
			.iter()
			.filter_map(|ev| ev.dyn_into::<web_sys::Event>().ok())
			.collect()
	}

	/// Show the body a returning editor's page hid: the first paint is the
	/// fork, so there is nothing published left to hide.
	#[cfg(target_arch = "wasm32")]
	pub fn reveal() {
		if let Some(style) =
			document_ext::document().get_element_by_id(Self::STYLE_ID)
		{
			style.remove();
		}
	}
}

/// When a served page loads its wasm: the page's boot policy, two of the
/// three a page chooses from (the third, never, is no `<Wasm>` at all).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default)]
pub enum BootPolicy {
	/// The module loader is in the page and runs as it parses: an app.
	#[default]
	Immediate,
	/// The loader is injected on the first click or toggle of an element
	/// marked [`PreBoot::INTENT`]: a page that is static until someone
	/// reaches for edit mode. A returning editor boots at once.
	Intent,
}

#[cfg(test)]
mod test {
	use super::*;

	/// The script carries the queue always, the bit only with a repo, and
	/// the loader only on intent.
	#[beet_core::test]
	fn the_script_carries_what_the_page_names() {
		PreBoot::script(None, None)
			.as_str()
			.xpect_contains("globalThis.beetPreBoot=")
			.xpect_contains("addEventListener(\"submit\",s,true)")
			.xnot()
			.xpect_contains("localStorage")
			.xnot()
			.xpect_contains("toggle");
		PreBoot::script(Some("beet:fork:http:repo"), None)
			.as_str()
			.xpect_contains("localStorage.getItem(\"beet:fork:http:repo\")")
			.xpect_contains("st.id=\"beet-pre-boot\"")
			.xnot()
			.xpect_contains("toggle");
		PreBoot::script(Some("beet:fork:http:repo"), Some("await 1;"))
			.as_str()
			.xpect_contains("m.textContent=\"await 1;\"")
			.xpect_contains("[data-beet-boot]")
			.xpect_contains("if(f)b();else{addEventListener(\"click\",i,true)");
	}
}
