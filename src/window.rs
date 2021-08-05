use crate::{TITLEBAR_HEIGHT, imports::*};
#[derive(Debug)]
pub struct RedWindow {
    pub window: Window,
    pub frame_window: Window,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl RedWindow {
    pub fn new(window: Window, frame_window: Window, geom: &GetGeometryReply) -> Self {
        RedWindow {
            window,
            frame_window,
            x: geom.x,
            y: geom.y,
            width: geom.width,
            height: geom.height,
        }
    }

    pub fn close_x_position(&self) -> i16 {
        std::cmp::max(0, self.width - TITLEBAR_HEIGHT) as _
    }
}
