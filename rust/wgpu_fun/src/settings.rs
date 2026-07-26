//! Settings store for examples with a GUI panel.
//!
//! The JS examples use the muigui library for their settings panels. Our
//! pages keep that same panel, but its onChange handlers call into the wasm
//! module (`set_setting_*` exports in web.rs), which lands here. The example's
//! frame callback reads current values with [`setting_f64`] & friends, and
//! changing a setting requests a redraw so `RenderMode::Once` examples update.
//!
//! Natively there is no panel; defaults are used, and the test harness can
//! override values via `WGPU_FUN_SETTING_<name>=<value>` environment
//! variables.

use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum SettingValue {
    Number(f64),
    Text(String),
    Bool(bool),
}

thread_local! {
    static SETTINGS: RefCell<HashMap<String, SettingValue>> = RefCell::new(HashMap::new());
    static REDRAW: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
}

pub fn set_setting(name: &str, value: SettingValue) {
    SETTINGS.with(|s| s.borrow_mut().insert(name.to_string(), value));
    request_redraw();
}

pub fn setting_f64(name: &str, default: f64) -> f64 {
    SETTINGS.with(|s| match s.borrow().get(name) {
        Some(SettingValue::Number(v)) => *v,
        Some(SettingValue::Bool(b)) => *b as u8 as f64,
        _ => default,
    })
}

pub fn setting_bool(name: &str, default: bool) -> bool {
    SETTINGS.with(|s| match s.borrow().get(name) {
        Some(SettingValue::Bool(b)) => *b,
        Some(SettingValue::Number(v)) => *v != 0.0,
        _ => default,
    })
}

pub fn setting_str(name: &str, default: &str) -> String {
    SETTINGS.with(|s| match s.borrow().get(name) {
        Some(SettingValue::Text(v)) => v.clone(),
        _ => default.to_string(),
    })
}

/// Ask the platform to render another frame (no-op in Continuous mode where
/// every frame renders anyway, and before the render loop starts).
pub fn request_redraw() {
    REDRAW.with(|r| {
        if let Some(f) = r.borrow().as_ref() {
            f();
        }
    });
}

pub(crate) fn set_redraw_hook(f: impl Fn() + 'static) {
    REDRAW.with(|r| *r.borrow_mut() = Some(Box::new(f)));
}

/// Native: seed settings from WGPU_FUN_SETTING_<name>=<value> env vars.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_settings_from_env() {
    for (key, value) in std::env::vars() {
        if let Some(name) = key.strip_prefix("WGPU_FUN_SETTING_") {
            let parsed = if let Ok(n) = value.parse::<f64>() {
                SettingValue::Number(n)
            } else if let Ok(b) = value.parse::<bool>() {
                SettingValue::Bool(b)
            } else {
                SettingValue::Text(value)
            };
            SETTINGS.with(|s| s.borrow_mut().insert(name.to_string(), parsed));
        }
    }
}
