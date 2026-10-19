use crate::anim::{Bezier, Tween};
use crate::geometry::{Point, Rect};
use crate::theme::Timing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    None,
    DragWindow,
    Minimize,
    Close,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Pointer {
    pub position: Point,
    pub pressed: bool,
}

pub struct Hover {
    hover: Tween,
    press: Tween,
    held: bool,
}

impl Hover {
    pub fn new() -> Self {
        Self {
            hover: Tween::at(0.0).over(Timing::FAST).eased(Bezier::EASE),
            press: Tween::at(0.0).over(Timing::FAST).eased(Bezier::EASE),
            held: false,
        }
    }

    pub fn track(&mut self, area: Rect, pointer: &Pointer) {
        let inside = area.contains(pointer.position);
        self.hover.set(if inside { 1.0 } else { 0.0 });
        self.press.set(if inside && self.held && pointer.pressed { 1.0 } else { 0.0 });
    }

    pub fn press(&mut self, area: Rect, pointer: &Pointer) -> bool {
        self.held = area.contains(pointer.position);
        self.press.set(if self.held { 1.0 } else { 0.0 });
        self.held
    }

    pub fn release(&mut self, area: Rect, pointer: &Pointer) -> bool {
        let clicked = self.held && area.contains(pointer.position);
        self.held = false;
        self.press.set(0.0);
        clicked
    }

    pub fn hover(&self) -> f32 {
        self.hover.value()
    }

    pub fn pressed(&self) -> f32 {
        self.press.value()
    }

    pub fn animating(&self) -> bool {
        self.hover.running() || self.press.running()
    }
}
