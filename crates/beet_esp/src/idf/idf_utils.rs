#![allow(non_upper_case_globals)]
use ::log::LevelFilter;
use anyhow::Result;
use esp_idf_svc::log::EspLogger;
use esp_idf_sys::*;
//https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/system/misc_system_api.html

pub fn b_to_kb(b: u32) -> String { format!("{:.2} KB", b as f32 / 1000.) }

pub fn print_free(prefix: &str) {
	print_free_heap(prefix);
	print_free_stack(prefix);
}

pub fn print_free_heap(prefix: &str) {
	log::info!(
		"\nHEAP - {prefix} - total: {}, internal: {}, largest block: {}",
		b_to_kb(free_heap_size()),
		b_to_kb(free_internal_heap_size()),
		b_to_kb(largest_block()),
	);
}

pub fn print_free_stack(prefix: &str) {
	log::info!("\nSTACK - {prefix} - total: {}", b_to_kb(free_stack_size()));
}

fn free_stack_size() -> u32 {
	unsafe { uxTaskGetStackHighWaterMark(std::ptr::null_mut()) }
}

pub fn init_esp() -> Result<()> {
	esp_idf_svc::sys::link_patches();
	init_logger()?;
	print_free("init");
	Ok(())
}

fn init_logger() -> Result<()> {
	EspLogger::initialize_default();
	set_esp_log_level(log::LevelFilter::Info)?;
	set_wifi_log_level(log::LevelFilter::Error)?;
	Ok(())
}

pub fn set_esp_log_level(level: LevelFilter) -> Result<()> {
	let logger = EspLogger;
	logger.set_target_level("*", level)?;
	// logger.initialize();
	Ok(())
}
pub fn set_wifi_log_level(level: LevelFilter) -> Result<()> {
	let logger = EspLogger;
	logger.set_target_level("wifi", level)?;
	// logger.initialize();
	Ok(())
}
//this is dumb
// pub fn dump_heap() {
//     unsafe { heap_caps_dump_all() };
// }

pub fn free_heap_size() -> u32 { unsafe { esp_get_free_heap_size() } }
pub fn free_internal_heap_size() -> u32 {
	unsafe { esp_get_free_internal_heap_size() }
}
pub fn largest_block() -> u32 {
	unsafe { heap_caps_get_largest_free_block(MALLOC_CAP_DEFAULT) as u32 }
}

pub fn was_ok_reset() -> bool {
	match reset_reason() {
		esp_reset_reason_t_ESP_RST_DEEPSLEEP => true,
		esp_reset_reason_t_ESP_RST_SW => true,
		esp_reset_reason_t_ESP_RST_POWERON => true,
		// esp_reset_reason_t_ESP_RST_STDIO => true, --any number?
		_ => false,
	}
}

pub fn reset_reason() -> esp_reset_reason_t { unsafe { esp_reset_reason() } }

// pub fn sleep_forever() -> ! {
// 	// foo::res
// 	loop {
// 		sleep_ms(16); //~60fps
// 	}
// }
#[allow(unreachable_code)]
pub fn restart() -> ! {
	unsafe { esp_restart() }
	//...actually never reached
	// sleep_forever();
}
