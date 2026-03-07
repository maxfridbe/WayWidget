use wayland_client::{protocol::{wl_keyboard, wl_surface}, Connection, QueueHandle};
use smithay_client_toolkit::seat::keyboard::{KeyEvent, KeyboardHandler, Modifiers};
use crate::WayWidget;

impl KeyboardHandler for WayWidget {
    fn enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &wl_keyboard::WlKeyboard, _surface: &wl_surface::WlSurface, _serial: u32, _raw_keys: &[u32], _keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym]) {}
    fn leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &wl_keyboard::WlKeyboard, _surface: &wl_surface::WlSurface, _serial: u32) {}
    fn press_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &wl_keyboard::WlKeyboard, _serial: u32, event: KeyEvent) {
        if *self.capture_keyboard.lock().unwrap() {
            let key_name = event.keysym.name().map(|s| s.to_string())
                .or_else(|| event.utf8.clone());
            if let Some(name) = key_name {
                self.keys_pressed.lock().unwrap().push(format!("+{}", name));
                self.needs_redraw = true;
                self.draw();
            }
        }
    }
    fn release_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &wl_keyboard::WlKeyboard, _serial: u32, event: KeyEvent) {
        if *self.capture_keyboard.lock().unwrap() {
            let key_name = event.keysym.name().map(|s| s.to_string())
                .or_else(|| event.utf8.clone());
            if let Some(name) = key_name {
                self.keys_pressed.lock().unwrap().push(format!("-{}", name));
                self.needs_redraw = true;
                self.draw();
            }
        }
    }
    fn repeat_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &wl_keyboard::WlKeyboard, _serial: u32, _event: KeyEvent) {}
    fn update_modifiers(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &wl_keyboard::WlKeyboard, _serial: u32, _modifiers: Modifiers, _raw_modifiers: smithay_client_toolkit::seat::keyboard::RawModifiers, _layout: u32) {}
}
