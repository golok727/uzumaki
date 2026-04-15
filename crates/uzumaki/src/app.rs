use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use winit::window::WindowId;

use crate::clipboard;
use crate::element::ElementTree;
use crate::event_dispatch;
use crate::gpu::GpuContext;
use crate::window;

pub struct WindowEntry {
    pub dom: ElementTree,
    pub handle: Option<window::Window>,
    pub rem_base: f32,
}

pub(crate) type WindowEntryId = u32;

pub struct AppState {
    pub gpu: GpuContext,
    pub windows: HashMap<WindowEntryId, WindowEntry>,
    pub winit_id_to_entry_id: HashMap<WindowId, WindowEntryId>,
    pub mouse_buttons: u8,
    pub modifiers: u32,
    pub clipboard: RefCell<clipboard::SystemClipboard>,
}

impl AppState {
    pub fn winit_window_id_to_entry_id(&self, window_id: &WindowId) -> Option<WindowEntryId> {
        self.winit_id_to_entry_id.get(window_id).cloned()
    }

    pub fn paint_window(&mut self, id: &WindowEntryId) {
        if let Some(window) = self.windows.get_mut(id)
            && let Some(handle) = &mut window.handle
        {
            handle.paint_and_present(&self.gpu.device, &self.gpu.queue, &mut window.dom);
        }
    }

    pub fn on_redraw_requested(&mut self, wid: &WindowEntryId) {
        if let Some(entry) = self.windows.get_mut(wid) {
            let WindowEntry { handle, dom, .. } = entry;
            if let Some(handle) = handle {
                event_dispatch::handle_redraw(dom, handle, &self.gpu.device, &self.gpu.queue);
                // handle.winit_window.request_redraw();
            }
        }
    }
    pub fn on_resize(&mut self, id: &WindowEntryId, width: u32, height: u32) -> bool {
        if let Some(window) = self.windows.get_mut(id)
            && let Some(handle) = &mut window.handle
            && handle.on_resize(&self.gpu.device, width, height)
        {
            handle.winit_window.request_redraw();
            return true;
        }
        false
    }
}

// Safety: We only access AppState from the main thread
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

pub(crate) type SharedAppState = Rc<RefCell<AppState>>;

pub(crate) fn with_state<R>(state: &SharedAppState, f: impl FnOnce(&mut AppState) -> R) -> R {
    f(&mut state.borrow_mut())
}
