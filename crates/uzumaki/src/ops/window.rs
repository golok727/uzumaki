use deno_core::*;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event_loop::EventLoopProxy;
use winit::window::{Fullscreen, Theme, Window as WinitWindow, WindowAttributes};

use crate::app::{
    SharedAppState, UserEvent, WindowEntry, WindowEntryId, with_state, with_state_ref,
};
use crate::style::*;
use crate::ui::UIState;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateWindowOptions {
    width: u32,
    height: u32,
    title: String,
    visible: Option<bool>,
    resizable: Option<bool>,
    decorations: Option<bool>,
    transparent: Option<bool>,
    maximized: Option<bool>,
    fullscreen: Option<bool>,
    min_width: Option<f64>,
    min_height: Option<f64>,
    max_width: Option<f64>,
    max_height: Option<f64>,
    position: Option<WindowPosition>,
    theme: Option<WindowTheme>,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowPosition {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum WindowTheme {
    Light,
    Dark,
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WindowSize {
    width: u32,
    height: u32,
}

impl WindowPosition {
    fn to_logical_position(self) -> LogicalPosition<f64> {
        LogicalPosition::new(self.x, self.y)
    }
}

impl WindowTheme {
    fn to_winit(self) -> Option<Theme> {
        match self {
            Self::Light => Some(Theme::Light),
            Self::Dark => Some(Theme::Dark),
            Self::System => None,
        }
    }

    fn from_winit(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self::Light,
            Theme::Dark => Self::Dark,
        }
    }
}

impl WindowSize {
    fn from_physical_size(size: winit::dpi::PhysicalSize<u32>) -> Self {
        Self {
            width: size.width,
            height: size.height,
        }
    }
}

impl CreateWindowOptions {
    pub(crate) fn to_window_attributes(&self) -> WindowAttributes {
        let mut attributes = WindowAttributes::default()
            .with_title(self.title.clone())
            .with_inner_size(LogicalSize::new(self.width as f64, self.height as f64));

        if let Some(visible) = self.visible {
            attributes = attributes.with_visible(visible);
        }
        if let Some(resizable) = self.resizable {
            attributes = attributes.with_resizable(resizable);
        }
        if let Some(decorations) = self.decorations {
            attributes = attributes.with_decorations(decorations);
        }
        if let Some(transparent) = self.transparent {
            attributes = attributes.with_transparent(transparent);
        }
        if let Some(maximized) = self.maximized {
            attributes = attributes.with_maximized(maximized);
        }
        if self.fullscreen == Some(true) {
            attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        if let Some(min_size) = try_logical_size(self.min_width, self.min_height) {
            attributes = attributes.with_min_inner_size(min_size);
        }
        if let Some(max_size) = try_logical_size(self.max_width, self.max_height) {
            attributes = attributes.with_max_inner_size(max_size);
        }
        if let Some(position) = self.position {
            attributes = attributes.with_position(position.to_logical_position());
        }
        if let Some(theme) = self.theme {
            attributes = attributes.with_theme(theme.to_winit());
        }

        attributes
    }
}

fn try_logical_size(width: Option<f64>, height: Option<f64>) -> Option<LogicalSize<f64>> {
    match (width, height) {
        (Some(width), Some(height))
            if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 =>
        {
            Some(LogicalSize::new(width, height))
        }
        _ => None,
    }
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
        .send_event(UserEvent::CreateWindow { id, options })
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

    fn with_window_entry<R>(
        &self,
        state: &OpState,
        f: impl FnOnce(&WindowEntry) -> R,
    ) -> Option<R> {
        let app = state.borrow::<SharedAppState>();
        with_state_ref(app, |state| state.windows.get(&self.id).map(f))
    }

    fn with_window_entry_mut<R>(
        &self,
        state: &OpState,
        f: impl FnOnce(&mut WindowEntry) -> R,
    ) -> Option<R> {
        let app = state.borrow::<SharedAppState>().clone();
        with_state(&app, |state| state.windows.get_mut(&self.id).map(f))
    }

    fn with_winit_window<R>(
        &self,
        state: &OpState,
        f: impl FnOnce(&WinitWindow) -> R,
    ) -> Option<R> {
        self.with_window_entry(state, |entry| {
            entry
                .handle
                .as_ref()
                .map(|handle| f(handle.winit_window.as_ref()))
        })
        .flatten()
    }

    fn with_winit_window_option<R>(
        &self,
        state: &OpState,
        f: impl FnOnce(&WinitWindow) -> Option<R>,
    ) -> Option<R> {
        self.with_winit_window(state, f).flatten()
    }

    fn update_winit_window(&self, state: &OpState, update: impl FnOnce(&WinitWindow)) -> bool {
        self.with_winit_window(state, update).is_some()
    }

    fn set_window_size_constraint(
        &self,
        state: &OpState,
        width: f64,
        height: f64,
        set_constraint: impl FnOnce(&WinitWindow, LogicalSize<f64>),
    ) -> bool {
        let Some(size) = try_logical_size(Some(width), Some(height)) else {
            return false;
        };

        self.with_winit_window(state, |window| set_constraint(window, size))
            .is_some()
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
#[allow(non_snake_case)]
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
    pub fn innerWidth(&self, state: &OpState) -> Option<u32> {
        self.with_window_entry(state, |entry| entry.inner_size().map(|(width, _)| width))
            .flatten()
    }

    /**
     * inner height of window in logical pixels
     */
    #[getter]
    pub fn innerHeight(&self, state: &OpState) -> Option<u32> {
        self.with_window_entry(state, |entry| entry.inner_size().map(|(_, height)| height))
            .flatten()
    }

    #[getter]
    #[string]
    pub fn title(&self, state: &OpState) -> Option<String> {
        self.with_winit_window(state, |window| window.title())
    }

    pub fn setTitle(&self, state: &OpState, #[string] title: String) -> bool {
        self.update_winit_window(state, |window| window.set_title(&title))
    }

    #[getter]
    pub fn visible(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window_option(state, |window| window.is_visible())
    }

    pub fn setVisible(&self, state: &OpState, visible: bool) -> bool {
        self.update_winit_window(state, |window| window.set_visible(visible))
    }

    #[getter]
    pub fn resizable(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window(state, |window| window.is_resizable())
    }

    pub fn setResizable(&self, state: &OpState, resizable: bool) -> bool {
        self.update_winit_window(state, |window| window.set_resizable(resizable))
    }

    #[getter]
    pub fn decorated(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window(state, |window| window.is_decorated())
    }

    pub fn setDecorations(&self, state: &OpState, decorations: bool) -> bool {
        self.update_winit_window(state, |window| window.set_decorations(decorations))
    }

    #[getter]
    pub fn maximized(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window(state, |window| window.is_maximized())
    }

    pub fn setMaximized(&self, state: &OpState, maximized: bool) -> bool {
        self.update_winit_window(state, |window| window.set_maximized(maximized))
    }

    #[getter]
    pub fn minimized(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window_option(state, |window| window.is_minimized())
    }

    pub fn setMinimized(&self, state: &OpState, minimized: bool) -> bool {
        self.update_winit_window(state, |window| window.set_minimized(minimized))
    }

    #[getter]
    pub fn fullscreen(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window(state, |window| window.fullscreen().is_some())
    }

    pub fn setFullscreen(&self, state: &OpState, fullscreen: bool) -> bool {
        self.update_winit_window(state, |window| {
            let target = fullscreen.then_some(Fullscreen::Borderless(None));
            window.set_fullscreen(target);
        })
    }

    pub fn setMinSize(&self, state: &OpState, width: f64, height: f64) -> bool {
        self.set_window_size_constraint(state, width, height, |window, size| {
            window.set_min_inner_size(Some(size));
        })
    }

    pub fn setMaxSize(&self, state: &OpState, width: f64, height: f64) -> bool {
        self.set_window_size_constraint(state, width, height, |window, size| {
            window.set_max_inner_size(Some(size));
        })
    }

    #[getter]
    #[serde]
    pub fn innerSize(&self, state: &OpState) -> Option<WindowSize> {
        self.with_winit_window(state, |window| {
            WindowSize::from_physical_size(window.inner_size())
        })
    }

    #[getter]
    #[serde]
    pub fn outerSize(&self, state: &OpState) -> Option<WindowSize> {
        self.with_winit_window(state, |window| {
            WindowSize::from_physical_size(window.outer_size())
        })
    }

    #[getter]
    #[serde]
    pub fn position(&self, state: &OpState) -> Option<WindowPosition> {
        self.with_winit_window_option(state, |window| {
            window.outer_position().ok().map(|position| WindowPosition {
                x: position.x as f64,
                y: position.y as f64,
            })
        })
    }

    pub fn setPosition(&self, state: &OpState, x: f64, y: f64) -> bool {
        if !x.is_finite() || !y.is_finite() {
            return false;
        }

        self.update_winit_window(state, |window| {
            window.set_outer_position(LogicalPosition::new(x, y));
        })
    }

    #[getter]
    pub fn scaleFactor(&self, state: &OpState) -> Option<f32> {
        self.with_window_entry(state, |entry| entry.scale_factor())
            .flatten()
    }

    #[getter]
    #[serde]
    pub fn theme(&self, state: &OpState) -> Option<WindowTheme> {
        self.with_winit_window_option(state, |window| window.theme().map(WindowTheme::from_winit))
    }

    pub fn setTheme(&self, state: &OpState, #[serde] theme: WindowTheme) -> bool {
        self.update_winit_window(state, |window| {
            window.set_theme(theme.to_winit());
        })
    }

    #[getter]
    pub fn remBase(&self, state: &OpState) -> f32 {
        self.with_window_entry(state, |entry| entry.rem_base)
            .unwrap_or(16.0)
    }

    #[setter]
    pub fn remBase(&self, state: &mut OpState, value: f64) {
        self.with_window_entry_mut(state, |entry| {
            entry.rem_base = value as f32;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{CreateWindowOptions, WindowPosition, WindowTheme};
    use winit::window::{Fullscreen, Theme};

    fn base_options() -> CreateWindowOptions {
        CreateWindowOptions {
            width: 800,
            height: 600,
            title: "demo".to_string(),
            visible: None,
            resizable: None,
            decorations: None,
            transparent: None,
            maximized: None,
            fullscreen: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            position: None,
            theme: None,
        }
    }

    #[test]
    fn create_options_map_common_attributes() {
        let mut options = base_options();
        options.visible = Some(false);
        options.resizable = Some(false);
        options.decorations = Some(false);
        options.transparent = Some(true);
        options.maximized = Some(true);
        options.fullscreen = Some(true);
        options.theme = Some(WindowTheme::Dark);

        let attributes = options.to_window_attributes();

        assert!(!attributes.visible);
        assert!(!attributes.resizable);
        assert!(!attributes.decorations);
        assert!(attributes.transparent);
        assert!(attributes.maximized);
        assert_eq!(attributes.preferred_theme, Some(Theme::Dark));
        assert!(matches!(
            attributes.fullscreen,
            Some(Fullscreen::Borderless(None))
        ));
    }

    #[test]
    fn create_options_map_sizes_and_position_when_complete() {
        let mut options = base_options();
        options.min_width = Some(320.0);
        options.min_height = Some(240.0);
        options.max_width = Some(1440.0);
        options.max_height = Some(900.0);
        options.position = Some(WindowPosition { x: 80.0, y: 120.0 });

        let attributes = options.to_window_attributes();

        assert!(attributes.min_inner_size.is_some());
        assert!(attributes.max_inner_size.is_some());
        assert!(attributes.position.is_some());
    }

    #[test]
    fn incomplete_size_constraints_are_ignored() {
        let mut options = base_options();
        options.min_width = Some(320.0);
        options.max_height = Some(900.0);

        let attributes = options.to_window_attributes();

        assert!(attributes.min_inner_size.is_none());
        assert!(attributes.max_inner_size.is_none());
    }

    #[test]
    fn system_theme_clears_preferred_theme() {
        let mut options = base_options();
        options.theme = Some(WindowTheme::System);

        let attributes = options.to_window_attributes();

        assert_eq!(attributes.preferred_theme, None);
    }
}
