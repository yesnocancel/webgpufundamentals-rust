//! Pointer input for examples that need it (camera controls, picking).
//!
//! Both platforms push events into a queue; the example's frame callback
//! drains it with [`drain_pointer_events`]. Coordinates are in device
//! pixels, matching `Frame::width`/`Frame::height` (the browser side
//! converts CSS pixels for you). Pushing an event also requests a redraw so
//! `RenderMode::Once` examples re-render on input.

use std::cell::RefCell;
use std::collections::VecDeque;

/// A pointer (mouse/touch/pen) or wheel event.
#[derive(Clone, Copy, Debug)]
pub enum PointerEvent {
    Down { x: f32, y: f32, button: u32 },
    Move { x: f32, y: f32 },
    Up { x: f32, y: f32, button: u32 },
    Wheel { delta_x: f32, delta_y: f32 },
}

thread_local! {
    static EVENTS: RefCell<VecDeque<PointerEvent>> = RefCell::new(VecDeque::new());
}

/// Take all pointer events received since the last call.
pub fn drain_pointer_events() -> Vec<PointerEvent> {
    EVENTS.with(|e| e.borrow_mut().drain(..).collect())
}

pub(crate) fn push_event(event: PointerEvent) {
    EVENTS.with(|e| e.borrow_mut().push_back(event));
    crate::settings::request_redraw();
}
