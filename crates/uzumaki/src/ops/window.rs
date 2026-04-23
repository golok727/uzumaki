use deno_core::*;
use winit::event_loop::EventLoopProxy;

use crate::app::{SharedAppState, UserEvent, WindowEntry, WindowEntryId, with_state};
use crate::style::*;
use crate::ui::UIState;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct CreateWindowOptions {
    width: u32,
    height: u32,
    title: String,
}

#[op2]
#[serde]
pub fn op_create_window(
    state: &mut OpState,
    #[serde] options: CreateWindowOptions,
) -> Result<WindowEntryId, deno_error::JsErrorBox> {
    static NEXT_WINDOW_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
    let id = NEXT_WINDOW_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let mut dom = UIState::new();
        let root = dom.create_view(UzStyle {
            display: Display::Flex,
            size: Size {
                width: Length::Percent(1.0),
                height: Length::Percent(1.0),
            },
            ..Default::default()
        });
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

    Ok(id)
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

#[op2(fast)]
pub fn op_set_rem_base(state: &mut OpState, #[smi] window_id: u32, value: f64) {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        if let Some(entry) = s.windows.get_mut(&window_id) {
            entry.rem_base = value as f32;
        }
    });
}

#[op2]
pub fn op_get_window_width(state: &mut OpState, #[smi] window_id: u32) -> Option<u32> {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        s.windows.get(&window_id).and_then(|entry| {
            entry.handle.as_ref().map(|h| {
                let size = h.winit_window.inner_size();
                size.width
            })
        })
    })
}

#[op2]
pub fn op_get_window_height(state: &mut OpState, #[smi] window_id: u32) -> Option<u32> {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        s.windows.get(&window_id).and_then(|entry| {
            entry.handle.as_ref().map(|h| {
                let size = h.winit_window.inner_size();
                size.height
            })
        })
    })
}

#[op2]
#[string]
pub fn op_get_window_title(state: &mut OpState, #[smi] window_id: u32) -> Option<String> {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        s.windows
            .get(&window_id)
            .and_then(|entry| entry.handle.as_ref().map(|h| h.winit_window.title()))
    })
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
