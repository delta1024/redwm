//! Holds the Window Manager State
use x11rb::CURRENT_TIME;
use crate::{
    define_window_event_request,
    imports::*,
    match_event_request,
    traits::*,
    RedWindow, TITLEBAR_HEIGHT, DRAG_BUTTON,
};
use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::HashSet;
use std::rc::Rc;
use x11rb::COPY_DEPTH_FROM_PARENT;

#[derive(Debug)]
pub struct RedWm<'a, C>
where
    C: Connection + Send + Sync,
{
    pub conn: Rc<RefCell<&'a C>>,
    pub screen_num: usize,
    pending_exposure: HashSet<Window>,
    sequences_to_ignore: BinaryHeap<Reverse<u16>>,
    windows: Vec<RedWindow>,
    black_gc: Gcontext,
    /** If this is Some, we are currently dragging the given window with the given offset relative
     to the mouse.
    */
    drag_window: Option<(Window, (i16, i16))>,
    wm_protocols: Atom,
    wm_delete_window: Atom,
}

impl<'a, C: 'static + Connection + Send + Sync> RedWm<'a, C> {
    pub fn new(conn: Rc<RefCell<&'a C>>, screen_num: usize) -> WmReply<Self> {
        let gc;
	let wm_protocols;
	let wm_delete_window;
        {
            let conn = conn.borrow();
            let screen = &conn.setup().roots[screen_num];
            gc = conn.generate_id()?;
            let font = conn.generate_id()?;
            conn.open_font(font, b"9x15")?;

            let gc_aux = CreateGCAux::new()
                .graphics_exposures(0)
                .background(screen.white_pixel)
                .foreground(screen.black_pixel)
                .font(font);
            conn.create_gc(gc, screen.root, &gc_aux)?;
            conn.close_font(font)?;
	    wm_protocols = conn.intern_atom(false, b"WM_PROTOCOLS")?;
	    wm_delete_window = conn.intern_atom(false, b"WM_DELETE_WINDOW")?;
        }

        let redwm = RedWm {
            conn,
            screen_num,
            black_gc: gc,
            pending_exposure: HashSet::default(),
            sequences_to_ignore: BinaryHeap::default(),
            windows: Vec::default(),
	    drag_window: Option::default(),
	    wm_protocols: wm_protocols.reply()?.atom,
	    wm_delete_window: wm_delete_window.reply()?.atom,
        };

        Ok(redwm)
    }
}

impl<C: Connection + Send + Sync> X11Connection for RedWm<'_, C> {
    fn get_screen<'c>(&'c self) -> WmReply<&'c Screen> {
        let screen;
        {
            let s = self.conn.borrow();
            screen = &s.setup().roots[self.screen_num];
        }

        Ok(screen)
    }

    fn scan(&mut self) -> WmReply<()> {
        let screen = self.get_screen()?;

        let my_cookies;
        {
            let conn = self.conn.borrow();
            let tree_reply = conn.query_tree(screen.root)?.reply()?;

            // For each window, request its atributes and geometry *now*
            let mut cookies = Vec::with_capacity(tree_reply.children.len());

            for win in tree_reply.children {
                let attr = conn.get_window_attributes(win)?;
                let geom = conn.get_geometry(win)?;
                cookies.push((win, attr, geom));
            }
            my_cookies = cookies;
        }

        for (win, attr, geom) in my_cookies {
            let (attr, geom) = (attr.reply(), geom.reply());

            if attr.is_err() || geom.is_err() {
                // just skip this window
                continue;
            }
            let (attr, geom) = (attr.unwrap(), geom.unwrap());

            if !attr.override_redirect && attr.map_state != MapState::UNMAPPED {
                self.manage_window(win, &geom)?;
            }
        }
        Ok(())
    }

    fn refresh(&mut self) {
        while let Some(&win) = self.pending_exposure.iter().next() {
            self.pending_exposure.remove(&win);
            if let Some(state) = self.find_window_by_id(win) {
                if let Err(err) = self.redraw_titlebar(state) {
                    eprintln!(
                        "Error while redrawing window {:x?}: {:?}",
                        state.window, err
                    );
                }
            }
        }
    }

    fn handle_event(&mut self, event: Event) -> WmReply<()> {
        let mut should_ignore = false;
        if let Some(seqno) = event.wire_sequence_number() {
            while let Some(&Reverse(to_ignore)) = self.sequences_to_ignore.peek() {
                if to_ignore.wrapping_sub(seqno) <= u16::max_value() / 2 {
                    should_ignore = to_ignore == seqno;
                    break;
                }
                self.sequences_to_ignore.pop();
            }
        }
        if should_ignore {
            return Ok(());
        }
        match_event_request!(self, event);
        Ok(())
    }

    fn manage_window(&mut self, win: Window, geom: &GetGeometryReply) -> WmReply<()> {
        let screen = self.get_screen()?;
        assert!(self.find_window_by_id(win).is_none());


        let conn = self.conn.borrow_mut();

        let frame_win = conn.generate_id()?;

        let win_aux = CreateWindowAux::new()
            .event_mask(define_window_event_request!())
            .background_pixel(screen.white_pixel);

        conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            frame_win,
            screen.root,
            geom.x,
            geom.y,
            geom.width,
            geom.height + TITLEBAR_HEIGHT,
            1,
            WindowClass::INPUT_OUTPUT,
            0,
            &win_aux,
        )?;

        conn.grab_server()?;
        conn.change_save_set(SetMode::INSERT, win)?;

        let cookie = conn.reparent_window(win, frame_win, 0, TITLEBAR_HEIGHT as _)?;

        conn.map_window(win)?;
        conn.map_window(frame_win)?;
        conn.ungrab_server()?;

        self.windows.push(RedWindow::new(win, frame_win, geom));

        self.sequences_to_ignore
            .push(Reverse(cookie.sequence_number() as u16));
        Ok(())
    }
}

impl<C: Connection + Send + Sync> ManageWindows for RedWm<'_, C> {
    fn find_window_by_id(&self, win: Window) -> Option<&RedWindow> {
        self.windows
            .iter()
            .find(|state| state.window == win || state.frame_window == win)
    }

    fn find_window_by_id_mut(&mut self, win: Window) -> Option<&mut RedWindow> {
        self.windows
            .iter_mut()
            .find(|state| state.window == win || state.frame_window == win)
    }

    fn redraw_titlebar(&self, state: &RedWindow) -> WmReply<()> {
        let close_x = state.close_x_position();
        let conn = self.conn.borrow();
        conn.poly_line(
            CoordMode::ORIGIN,
            state.frame_window,
            self.black_gc,
            &[
                Point {
                    x: close_x,
                    y: TITLEBAR_HEIGHT as _,
                },
                Point {
                    x: state.width as _,
                    y: 0,
                },
            ],
        )?;

        let reply = conn
            .get_property(
                false,
                state.window,
                AtomEnum::WM_NAME,
                AtomEnum::STRING,
                0,
                std::u32::MAX,
            )?
            .reply()?;
        conn.image_text8(state.frame_window, self.black_gc, 1, 10, &reply.value)?;
        Ok(())
    }
}

impl<C: Connection + Send + Sync> HandleEvent for RedWm<'_, C> {
    fn handle_unmap_notify(&mut self, event: UnmapNotifyEvent) -> WmReply<()> {
        let root = self.get_screen()?.root;
        let conn = self.conn.borrow();

        self.windows.retain(|state| {
            if state.window != event.window {
                return true;
            }
            conn.change_save_set(SetMode::DELETE, state.window).unwrap();
            conn.reparent_window(state.window, root, state.x, state.y)
                .unwrap();
            conn.destroy_window(state.frame_window).unwrap();
            false
        });
        Ok(())
    }

    fn handle_map_request(&mut self, event: MapRequestEvent) -> WmReply<()> {
        let conn = self.conn.borrow().clone();
        self.manage_window(event.window, &conn.get_geometry(event.window)?.reply()?)?;
        Ok(())
    }

    fn handle_configure_request(&mut self, event: ConfigureRequestEvent) -> WmReply<()> {
        if let Some(state) = self.find_window_by_id_mut(event.window) {
            let _ = state;
            unimplemented!();
        }

        let aux = ConfigureWindowAux::from_configure_request(&event)
            .sibling(None)
            .stack_mode(None);
        self.conn.borrow().configure_window(event.window, &aux)?;
        Ok(())
    }

    fn handle_expose(&mut self, event: ExposeEvent) {
	self.pending_exposure.insert(event.window);
    }

    fn handle_enter(&mut self, event: EnterNotifyEvent) -> WmReply<()> {
	let conn = self.conn.borrow();

	if let Some(state) = self.find_window_by_id(event.event) {
            // Set the input focus (ignoring ICCCM's WM_PROTOCOLS / WM_TAKE_FOCUS)
	    conn.set_input_focus(InputFocus::PARENT, state.window, CURRENT_TIME)?;
            // Also raise the window to the top of the stacking order
	    conn.configure_window(
		state.frame_window,
		&ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
	    )?;
	}
	Ok(())
    }

    fn handle_button_press(&mut self, event: ButtonPressEvent) {
	if event.detail != DRAG_BUTTON  || event.state != 0 {
	    return;
	}

	if let Some(state) = self.find_window_by_id(event.event) {
	    let (x, y) = (-event.event_x, -event.event_y);
	    self.drag_window = Some((state.frame_window, (x, y)));
	}
	
    }

    fn handle_button_release(&mut self, event: ButtonReleaseEvent) -> WmReply<()> {
	if event.detail == DRAG_BUTTON {
	    self.drag_window = None;
	}
	if let Some(state) = self.find_window_by_id(event.event) {
	    if event.event_x >= state.close_x_position() {
		let data = [self.wm_delete_window, 0, 0, 0, 0];
		let event = ClientMessageEvent {
		    response_type: CLIENT_MESSAGE_EVENT,
		    format: 32,
		    sequence: 0,
		    window: state.window,
		    type_: self.wm_protocols,
		    data: data.into(),
		};
		self.conn.borrow().send_event(false, state.window, EventMask::NO_EVENT, &event)?;
	    }
	}
	Ok(())
    }

    fn handle_motion_notify(&mut self, event: MotionNotifyEvent) -> WmReply<()> {
	if let Some((win, (x, y))) = self.drag_window {
	    let (x, y) = (x + event.root_x, y + event.root_x);

	    let (x, y) =  (x as i32, y as i32);
	    self.conn.borrow().configure_window(win, &ConfigureWindowAux::new().x(x).y(y))?;
	}
	Ok(())
    }
}
