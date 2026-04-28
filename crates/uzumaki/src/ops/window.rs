use deno_core::*;
use winit::event_loop::EventLoopProxy;

use crate::app::{
    SharedAppState, UserEvent, WindowEntry, WindowEntryId, with_state, with_state_ref,
};
use crate::style::*;
use crate::ui::UIState;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct CreateWindowOptions {
    width: u32,
    height: u32,
    title: String,
}

#[op2]
#[cppgc]
pub fn op_create_window(
    state: &mut OpState,
    #[serde] options: CreateWindowOptions,
) -> Result<CoreWindow, deno_error::JsErrorBox> {
    static NEXT_WINDOW_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
    let id = NEXT_WINDOW_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let mut dom = UIState::new();
        let root = dom.create_view(UzStyle::root());
        dom.set_root(root);

        s.windows.insert(
            id,
            WindowEntry {
                dom,
                handle: None,
                rem_base: 16.0,
                cursor_blink_generation: 0,
            },
        );
    });

    let proxy = state.borrow::<EventLoopProxy<UserEvent>>();
    proxy
        .send_event(UserEvent::CreateWindow {
            id,
            width: options.width,
            height: options.height,
            title: options.title,
        })
        .map_err(|_| {
            deno_error::JsErrorBox::new(
                "UzumakiInternalError",
                "cannot create window after application free",
            )
        })?;

    Ok(CoreWindow::new(id))
}

#[op2(fast)]
pub fn op_request_quit(state: &mut OpState) -> Result<(), deno_error::JsErrorBox> {
    let proxy = state.borrow::<EventLoopProxy<UserEvent>>();
    proxy
        .send_event(UserEvent::Quit)
        .map_err(|_| deno_error::JsErrorBox::new("UzumakiInternalError", "error quitting"))?;
    Ok(())
}

#[op2(fast)]
pub fn op_request_redraw(
    state: &mut OpState,
    #[smi] window_id: u32,
) -> Result<(), deno_error::JsErrorBox> {
    let proxy = state.borrow::<EventLoopProxy<UserEvent>>();
    proxy
        .send_event(UserEvent::RequestRedraw { id: window_id })
        .map_err(|_| {
            deno_error::JsErrorBox::new("UzumakiInternalError", "error requesting redraw")
        })?;
    Ok(())
}

#[op2]
#[string]
pub fn op_read_clipboard_text(state: &mut OpState) -> Option<String> {
    let app_state = state.borrow::<SharedAppState>().clone();
    let s = app_state.borrow();
    match s.clipboard.borrow_mut().read_text() {
        Ok(text) => text,
        Err(e) => {
            eprintln!("[uzumaki] clipboard read error: {e}");
            None
        }
    }
}

#[op2(fast)]
pub fn op_write_clipboard_text(state: &mut OpState, #[string] text: String) -> bool {
    let app_state = state.borrow::<SharedAppState>().clone();
    let s = app_state.borrow();
    match s.clipboard.borrow_mut().write_text(&text) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("[uzumaki] clipboard write error: {e}");
            false
        }
    }
}

use deno_core::GarbageCollected;

pub struct CoreWindow {
    id: WindowEntryId,
}

impl CoreWindow {
    pub fn new(id: WindowEntryId) -> Self {
        Self { id }
    }
}

// SAFETY: we're sure this can be GCed
unsafe impl GarbageCollected for CoreWindow {
    fn trace(&self, _visitor: &mut deno_core::v8::cppgc::Visitor) {}

    fn get_name(&self) -> &'static std::ffi::CStr {
        c"CoreWindow"
    }
}

#[op2]
impl CoreWindow {
    #[getter]
    pub fn id(&self) -> WindowEntryId {
        self.id
    }

    #[fast]
    pub fn close(&self, state: &OpState) -> Result<(), deno_error::JsErrorBox> {
        let proxy = state.borrow::<EventLoopProxy<UserEvent>>();

        proxy
            .send_event(UserEvent::CloseWindow { id: self.id })
            .map_err(|_| {
                deno_error::JsErrorBox::new("UzumakiInternalError", "error closing window")
            })?;
        Ok(())
    }

    /**
     * inner width of window in logical pixels
     */
    #[getter]
    #[allow(non_snake_case)]
    pub fn innerWidth(&self, state: &OpState) -> Option<u32> {
        let app = state.borrow::<SharedAppState>();

        with_state_ref(app, |state| {
            state
                .windows
                .get(&self.id)
                .and_then(|w| w.inner_size().map(|(w, _)| w))
        })
    }

    #[getter]
    #[allow(non_snake_case)]
    pub fn scaleFactor(&self, state: &OpState) -> Option<f32> {
        let app = state.borrow::<SharedAppState>();

        with_state_ref(app, |state| {
            state.windows.get(&self.id).and_then(|w| w.scale_factor())
        })
    }

    /**
     * inner height of window in logical pixels
     */
    #[getter]
    #[allow(non_snake_case)]
    pub fn innerHeight(&self, state: &OpState) -> Option<u32> {
        let app = state.borrow::<SharedAppState>();

        with_state_ref(app, |state| {
            state
                .windows
                .get(&self.id)
                .and_then(|w| w.inner_size().map(|(_, h)| h))
        })
    }

    #[getter]
    #[string]
    pub fn title(&self, state: &OpState) -> Option<String> {
        let app = state.borrow::<SharedAppState>();

        with_state_ref(app, |state| {
            state.windows.get(&self.id).and_then(|entry| {
                entry
                    .handle
                    .as_ref()
                    .map(|handle| handle.winit_window.title())
            })
        })
    }

    #[getter]
    #[allow(non_snake_case)]
    pub fn remBase(&self, state: &OpState) -> f32 {
        let app = state.borrow::<SharedAppState>();

        with_state_ref(app, |state| {
            state
                .windows
                .get(&self.id)
                .map(|w| w.rem_base)
                .unwrap_or(16.0)
        })
    }

    #[setter]
    #[allow(non_snake_case)]
    pub fn remBase(&self, state: &mut OpState, value: f64) {
        let app = state.borrow_mut::<SharedAppState>();
        with_state(app, |state| {
            if let Some(entry) = state.windows.get_mut(&self.id) {
                entry.rem_base = value as f32;
            }
        });
    }
}
