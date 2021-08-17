/*! RedWm
RedWm is a tiling window manager for GNU/Linux using the [`x11rb`] crate.
# Todo
* TESTING refactor wm to use trates
* Keybindings
* window resize
## Planed Features
* Hooks
* Layouts
* Workspaces
* AutoStart
* Window Spawn Rules
* RedBar integration
*/
#[allow(unused_imports)]
pub mod imports {
    pub use std::process::exit;
    pub use x11rb::{
        self,
        connection::Connection,
        errors::{ReplyError, ReplyOrIdError},
        protocol::{xproto::*, ErrorKind, Event},
    };
}
pub mod traits {
    use std::error::Error;
    pub type WmReply<T> = Result<T, Box<dyn Error>>;
    pub use super::x11::traits::*;
}

/// Traits for interacting with the x11 server
pub mod x11;
use imports::*;
use std::cell::RefCell;
use std::rc::Rc;
use traits::*;
pub mod core;
#[doc(inline)]
pub use crate::core as wm_core;

pub const TITLEBAR_HEIGHT: u16 = 0;
pub const DRAG_BUTTON: Button = 1;
pub const USER_FONT: &str = "FiraCode Nerd Font-12";

pub fn run<C>(x_reply: (C, usize)) -> WmReply<()>
where
    C: Connection + 'static + Send + Sync,
{
    let (conn, screen_num) = x_reply;
    let screen = &conn.setup().roots[screen_num];
    x11::check_access(&conn, &screen)?;
    let conn = Rc::new(RefCell::new(&conn));
    let mut wm = wm_core::RedWm::new(conn, screen_num)?;
    wm.scan()?;
    run_event_loop(&mut wm)?;
    Ok(())
}
/// Primary Event Loop
#[allow(unreachable_code)]
fn run_event_loop<C>(wm: &mut wm_core::RedWm<C>) -> WmReply<()>
where
    C: Connection + Send + Sync,
{
    let wm = Rc::new(RefCell::new(wm));

    loop {
        {
            wm.borrow_mut().refresh();
        }
        let event;
        {
            let wm = wm.borrow();
            let conn = wm.conn.borrow();
            conn.flush()?;
            event = conn.wait_for_event()?;
        }
        let mut event_option = Some(event);

        while let Some(event) = event_option {
            if let Event::ClientMessage(_) = event {
                // may cause exiting unnesisarily
                return Ok(());
            }
            {
                wm.borrow_mut().handle_event(event)?;
            }
            {
                let wm = wm.borrow();
                let conn = wm.conn.borrow();
                event_option = conn.poll_for_event()?;
            }
        }
    }
    // unreachable code
    Ok(())
}

#[macro_export]
macro_rules! define_event_request {
    () => {
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_REDIRECT
    };
}
#[macro_export]
macro_rules! define_window_event_request {
    () => {
        EventMask::EXPOSURE
            | EventMask::SUBSTRUCTURE_NOTIFY
            | EventMask::BUTTON_PRESS
            | EventMask::BUTTON_RELEASE
            | EventMask::POINTER_MOTION
            | EventMask::ENTER_WINDOW
    };
}
#[macro_export]
macro_rules! match_event_request {
    ($self: tt, $event: tt) => {
        match $event {
            Event::UnmapNotify(event) => $self.handle_unmap_notify(event)?,
            Event::ConfigureRequest(event) => $self.handle_configure_request(event)?,
            Event::MapRequest(event) => $self.handle_map_request(event)?,
            Event::Expose(event) => $self.handle_expose(event),
            Event::EnterNotify(event) => $self.handle_enter(event)?,
            Event::ButtonPress(event) => $self.handle_button_press(event),
            Event::ButtonRelease(event) => $self.handle_button_release(event)?,
            Event::MotionNotify(event) => $self.handle_motion_notify(event)?,
            _ => {}
        }
    };
}
