use deno_core::*;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event_loop::EventLoopProxy;
use winit::window::{
    Fullscreen, Theme, Window as WinitWindow, WindowAttributes, WindowButtons, WindowLevel,
};

use crate::app::{
    SharedAppState, UserEvent, WindowEntry, WindowEntryId, with_state, with_state_ref,
};
use crate::style::*;
use crate::ui::UIState;

const DEFAULT_MIN_WINDOW_WIDTH: f64 = 400.0;
const DEFAULT_MIN_WINDOW_HEIGHT: f64 = 300.0;

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
    minimized: Option<bool>,
    fullscreen: Option<bool>,
    always_on_top: Option<bool>,
    window_level: Option<UzWindowLevel>,
    min_width: Option<f64>,
    min_height: Option<f64>,
    max_width: Option<f64>,
    max_height: Option<f64>,
    position: Option<WindowPosition>,
    theme: Option<WindowTheme>,
    active: Option<bool>,
    content_protected: Option<bool>,
    enabled_buttons: Option<EnabledWindowButtons>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
enum UzWindowLevel {
    Normal,
    AlwaysOnTop,
    AlwaysOnBottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnabledWindowButtons {
    close: Option<bool>,
    minimize: Option<bool>,
    maximize: Option<bool>,
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

impl UzWindowLevel {
    fn to_winit(self) -> WindowLevel {
        match self {
            Self::Normal => WindowLevel::Normal,
            Self::AlwaysOnTop => WindowLevel::AlwaysOnTop,
            Self::AlwaysOnBottom => WindowLevel::AlwaysOnBottom,
        }
    }

    fn from_winit(level: WindowLevel) -> Self {
        match level {
            WindowLevel::Normal => Self::Normal,
            WindowLevel::AlwaysOnTop => Self::AlwaysOnTop,
            WindowLevel::AlwaysOnBottom => Self::AlwaysOnBottom,
        }
    }
}

impl EnabledWindowButtons {
    fn all_enabled() -> Self {
        Self {
            close: Some(true),
            minimize: Some(true),
            maximize: Some(true),
        }
    }

    fn to_winit(self) -> WindowButtons {
        let mut buttons = WindowButtons::empty();
        if self.close.unwrap_or(true) {
            buttons |= WindowButtons::CLOSE;
        }
        if self.minimize.unwrap_or(true) {
            buttons |= WindowButtons::MINIMIZE;
        }
        if self.maximize.unwrap_or(true) {
            buttons |= WindowButtons::MAXIMIZE;
        }
        buttons
    }

    fn from_winit(buttons: WindowButtons) -> Self {
        Self {
            close: Some(buttons.contains(WindowButtons::CLOSE)),
            minimize: Some(buttons.contains(WindowButtons::MINIMIZE)),
            maximize: Some(buttons.contains(WindowButtons::MAXIMIZE)),
        }
    }
}

impl WindowSize {
    fn from_physical_size(size: winit::dpi::PhysicalSize<u32>, scale_factor: f64) -> Self {
        let size: LogicalSize<u32> = size.to_logical(scale_factor);
        Self {
            width: size.width,
            height: size.height,
        }
    }
}

impl CreateWindowOptions {
    pub(crate) fn transparent(&self) -> bool {
        self.transparent.unwrap_or(false)
    }

    pub(crate) fn minimized(&self) -> bool {
        self.minimized.unwrap_or(false)
    }

    pub(crate) fn content_protected(&self) -> bool {
        self.content_protected.unwrap_or(false)
    }

    pub(crate) fn window_level(&self) -> WindowLevel {
        match (self.window_level, self.always_on_top) {
            (Some(level), _) => level.to_winit(),
            (None, Some(true)) => WindowLevel::AlwaysOnTop,
            (None, Some(false)) | (None, None) => WindowLevel::Normal,
        }
    }

    pub(crate) fn enabled_buttons(&self) -> WindowButtons {
        self.enabled_buttons
            .unwrap_or_else(EnabledWindowButtons::all_enabled)
            .to_winit()
    }

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
        attributes = attributes.with_window_level(self.window_level());
        if self.fullscreen == Some(true) {
            attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        let default_min_size =
            LogicalSize::new(DEFAULT_MIN_WINDOW_WIDTH, DEFAULT_MIN_WINDOW_HEIGHT);
        if let Some(min_size) = try_logical_size(self.min_width, self.min_height) {
            attributes = attributes.with_min_inner_size(min_size);
        } else {
            attributes = attributes.with_min_inner_size(default_min_size);
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
        if let Some(active) = self.active {
            attributes = attributes.with_active(active);
        }
        if let Some(content_protected) = self.content_protected {
            attributes = attributes.with_content_protected(content_protected);
        }
        if let Some(enabled_buttons) = self.enabled_buttons {
            attributes = attributes.with_enabled_buttons(enabled_buttons.to_winit());
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
                transparent: options.transparent(),
                window_level: options.window_level(),
                content_protected: options.content_protected(),
                enabled_buttons: options.enabled_buttons(),
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

    fn set_window_level_state(&self, state: &OpState, level: WindowLevel) -> bool {
        self.with_window_entry_mut(state, |entry| {
            entry.window_level = level;
            if let Some(handle) = entry.handle.as_ref() {
                handle.winit_window.set_window_level(level);
            }
        })
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

    #[fast]
    pub fn setTitle(&self, state: &OpState, #[string] title: String) -> bool {
        self.update_winit_window(state, |window| window.set_title(&title))
    }

    #[getter]
    pub fn visible(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window_option(state, |window| window.is_visible())
    }

    #[fast]
    pub fn setVisible(&self, state: &OpState, visible: bool) -> bool {
        self.update_winit_window(state, |window| window.set_visible(visible))
    }

    #[getter]
    pub fn transparent(&self, state: &OpState) -> Option<bool> {
        self.with_window_entry(state, |entry| entry.transparent)
    }

    #[fast]
    pub fn setTransparent(&self, state: &OpState, transparent: bool) -> bool {
        self.with_window_entry_mut(state, |entry| {
            entry.transparent = transparent;
            if let Some(handle) = entry.handle.as_mut() {
                handle.set_transparent(transparent);
            }
        })
        .is_some()
    }

    #[getter]
    pub fn resizable(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window(state, |window| window.is_resizable())
    }

    #[fast]
    pub fn setResizable(&self, state: &OpState, resizable: bool) -> bool {
        self.update_winit_window(state, |window| window.set_resizable(resizable))
    }

    #[getter]
    pub fn decorated(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window(state, |window| window.is_decorated())
    }

    #[fast]
    pub fn setDecorations(&self, state: &OpState, decorations: bool) -> bool {
        self.update_winit_window(state, |window| window.set_decorations(decorations))
    }

    #[getter]
    pub fn maximized(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window(state, |window| window.is_maximized())
    }

    #[fast]
    pub fn setMaximized(&self, state: &OpState, maximized: bool) -> bool {
        self.update_winit_window(state, |window| window.set_maximized(maximized))
    }

    #[getter]
    pub fn minimized(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window_option(state, |window| window.is_minimized())
    }

    #[fast]
    pub fn setMinimized(&self, state: &OpState, minimized: bool) -> bool {
        self.update_winit_window(state, |window| window.set_minimized(minimized))
    }

    #[getter]
    pub fn fullscreen(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window(state, |window| window.fullscreen().is_some())
    }

    #[fast]
    pub fn setFullscreen(&self, state: &OpState, fullscreen: bool) -> bool {
        self.update_winit_window(state, |window| {
            let target = fullscreen.then_some(Fullscreen::Borderless(None));
            window.set_fullscreen(target);
        })
    }

    #[getter]
    pub fn alwaysOnTop(&self, state: &OpState) -> Option<bool> {
        self.with_window_entry(state, |entry| {
            entry.window_level == WindowLevel::AlwaysOnTop
        })
    }

    #[fast]
    pub fn setAlwaysOnTop(&self, state: &OpState, always_on_top: bool) -> bool {
        let level = if always_on_top {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        };
        self.set_window_level_state(state, level)
    }

    #[getter]
    #[serde]
    pub fn windowLevel(&self, state: &OpState) -> Option<UzWindowLevel> {
        self.with_window_entry(state, |entry| UzWindowLevel::from_winit(entry.window_level))
    }

    pub fn setWindowLevel(&self, state: &OpState, #[serde] level: UzWindowLevel) -> bool {
        self.set_window_level_state(state, level.to_winit())
    }

    #[fast]
    pub fn setMinSize(&self, state: &OpState, width: f64, height: f64) -> bool {
        self.set_window_size_constraint(state, width, height, |window, size| {
            window.set_min_inner_size(Some(size));
        })
    }

    #[fast]
    pub fn setMaxSize(&self, state: &OpState, width: f64, height: f64) -> bool {
        self.set_window_size_constraint(state, width, height, |window, size| {
            window.set_max_inner_size(Some(size));
        })
    }

    #[getter]
    #[serde]
    pub fn innerSize(&self, state: &OpState) -> Option<WindowSize> {
        self.with_winit_window(state, |window| {
            WindowSize::from_physical_size(window.inner_size(), window.scale_factor())
        })
    }

    #[getter]
    #[serde]
    pub fn outerSize(&self, state: &OpState) -> Option<WindowSize> {
        self.with_winit_window(state, |window| {
            WindowSize::from_physical_size(window.outer_size(), window.scale_factor())
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

    #[fast]
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
    pub fn active(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window(state, |window| window.has_focus())
    }

    #[fast]
    pub fn focus(&self, state: &OpState) -> bool {
        self.update_winit_window(state, |window| window.focus_window())
    }

    #[getter]
    pub fn contentProtected(&self, state: &OpState) -> Option<bool> {
        self.with_window_entry(state, |entry| entry.content_protected)
    }

    #[fast]
    pub fn setContentProtected(&self, state: &OpState, protected: bool) -> bool {
        self.with_window_entry_mut(state, |entry| {
            entry.content_protected = protected;
            if let Some(handle) = entry.handle.as_ref() {
                handle.winit_window.set_content_protected(protected);
            }
        })
        .is_some()
    }

    #[getter]
    #[serde]
    pub fn enabledButtons(&self, state: &OpState) -> Option<EnabledWindowButtons> {
        self.with_window_entry(state, |entry| {
            EnabledWindowButtons::from_winit(entry.enabled_buttons)
        })
    }

    pub fn setEnabledButtons(
        &self,
        state: &OpState,
        #[serde] buttons: EnabledWindowButtons,
    ) -> bool {
        let buttons = buttons.to_winit();
        self.with_window_entry_mut(state, |entry| {
            entry.enabled_buttons = buttons;
            if let Some(handle) = entry.handle.as_ref() {
                handle.winit_window.set_enabled_buttons(buttons);
            }
        })
        .is_some()
    }

    #[getter]
    pub fn remBase(&self, state: &OpState) -> f32 {
        self.with_window_entry(state, |entry| entry.rem_base)
            .unwrap_or(16.0)
    }

    #[fast]
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
            minimized: None,
            fullscreen: None,
            always_on_top: None,
            window_level: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            position: None,
            theme: None,
            active: None,
            content_protected: None,
            enabled_buttons: None,
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
        options.active = Some(true);
        options.content_protected = Some(true);

        let attributes = options.to_window_attributes();

        assert!(!attributes.visible);
        assert!(!attributes.resizable);
        assert!(!attributes.decorations);
        assert!(attributes.transparent);
        assert!(attributes.maximized);
        assert!(attributes.active);
        assert!(attributes.content_protected);
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
    fn default_min_size_is_preserved_without_explicit_min_size() {
        let options = base_options();

        let attributes = options.to_window_attributes();

        assert!(attributes.min_inner_size.is_some());
    }

    #[test]
    fn incomplete_size_constraints_do_not_override_defaults() {
        let mut options = base_options();
        options.min_width = Some(320.0);
        options.max_height = Some(900.0);

        let attributes = options.to_window_attributes();

        assert!(attributes.min_inner_size.is_some());
        assert!(attributes.max_inner_size.is_none());
    }

    #[test]
    fn system_theme_clears_preferred_theme() {
        let mut options = base_options();
        options.theme = Some(WindowTheme::System);

        let attributes = options.to_window_attributes();

        assert_eq!(attributes.preferred_theme, None);
    }

    #[test]
    fn always_on_top_maps_to_window_level_when_no_explicit_level() {
        let mut options = base_options();
        options.always_on_top = Some(true);

        assert_eq!(
            options.window_level(),
            winit::window::WindowLevel::AlwaysOnTop
        );
        assert_eq!(
            options.to_window_attributes().window_level,
            winit::window::WindowLevel::AlwaysOnTop
        );
    }

    #[test]
    fn explicit_window_level_wins_over_always_on_top() {
        let mut options = base_options();
        options.always_on_top = Some(true);
        options.window_level = Some(super::UzWindowLevel::AlwaysOnBottom);

        assert_eq!(
            options.window_level(),
            winit::window::WindowLevel::AlwaysOnBottom
        );
    }

    #[test]
    fn enabled_buttons_default_missing_fields_to_enabled() {
        let mut options = base_options();
        options.enabled_buttons = Some(super::EnabledWindowButtons {
            close: Some(false),
            minimize: None,
            maximize: Some(true),
        });

        let buttons = options.enabled_buttons();

        assert!(!buttons.contains(winit::window::WindowButtons::CLOSE));
        assert!(buttons.contains(winit::window::WindowButtons::MINIMIZE));
        assert!(buttons.contains(winit::window::WindowButtons::MAXIMIZE));
        assert_eq!(options.to_window_attributes().enabled_buttons, buttons);
    }

    #[test]
    fn minimized_is_post_create_state() {
        let mut options = base_options();
        options.minimized = Some(true);

        assert!(options.minimized());
    }
}
