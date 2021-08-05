use super::imports::*;
use crate::define_event_request;


pub mod traits {
    use crate::imports::*;
    use crate::traits::*;
    use crate::RedWindow;

    /// Functions for managing the X Connection
    pub trait X11Connection {
        fn handle_event(&mut self, event: Event) -> WmReply<()>;
        fn scan(&mut self) -> WmReply<()>;
        fn get_screen<'c>(&'c self) -> WmReply<&'c Screen>;
        /// Do all the pending work that was queued while handling some events
        fn refresh(&mut self);
        fn manage_window(
            &mut self,
            win: Window,
            goem: &GetGeometryReply,
        ) -> WmReply<()>;
    }

    /// Functions for managing windows
    pub trait ManageWindows: X11Connection {
        fn find_window_by_id(&self, win: Window) -> Option<&RedWindow>;
        fn find_window_by_id_mut(&mut self, win: Window) -> Option<&mut RedWindow>;
        fn redraw_titlebar(&self, state: &RedWindow) -> WmReply<()>;
    }

    /// Functions for handling events
    pub trait HandleEvent: ManageWindows {
        fn handle_unmap_notify(&mut self, event: UnmapNotifyEvent) -> WmReply<()>;
        fn handle_map_request(&mut self, event: MapRequestEvent) -> WmReply<()>;
        fn handle_configure_request(&mut self, event: ConfigureRequestEvent) -> WmReply<()>;
        fn handle_expose(&mut self, event: ExposeEvent);
        fn handle_enter(&mut self, event: EnterNotifyEvent) -> WmReply<()>;
        fn handle_button_press(&mut self, event: ButtonPressEvent);
        fn handle_button_release(&mut self, event: ButtonReleaseEvent) -> WmReply<()>;
        fn handle_motion_notify(&mut self, event: MotionNotifyEvent) -> WmReply<()>;
    }
}
pub fn check_access<C: Connection>(conn: &C, screen: &Screen) -> Result<(), ReplyError> {
    let change = ChangeWindowAttributesAux::default().event_mask(define_event_request!());
    let res = conn.change_window_attributes(screen.root, &change)?.check();

    if let Err(ReplyError::X11Error(ref error)) = res {
        if error.error_kind == ErrorKind::Access {
            eprintln!("Another WM is already running.");
            exit(1);
        } else {
            res
        }
    } else {
        res
    }
}
