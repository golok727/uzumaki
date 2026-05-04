use deno_core::*;
use refineable::Refineable;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition};
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

#[derive(Clone, Debug, PartialEq, Refineable, serde::Deserialize)]
#[refineable(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WindowOptions {
    width: u32,
    height: u32,
    title: String,
    visible: bool,
    resizable: bool,
    decorations: bool,
    transparent: bool,
    maximized: bool,
    minimized: bool,
    fullscreen: bool,
    window_level: Option<UzWindowLevel>,
    min_width: Option<f64>,
    min_height: Option<f64>,
    max_width: Option<f64>,
    max_height: Option<f64>,
    position: Option<WindowPosition>,
    theme: Option<WindowTheme>,
    active: Option<bool>,
    content_protected: bool,
    closable: bool,
    minimizable: bool,
    maximizable: bool,
}

pub(crate) type CreateWindowOptions = WindowOptionsRefinement;

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

impl UzWindowLevel {
    fn as_str(&self) -> &str {
        match self {
            UzWindowLevel::Normal => "normal",
            UzWindowLevel::AlwaysOnTop => "alwaysOnTop",
            UzWindowLevel::AlwaysOnBottom => "alwaysOnBottom",
        }
    }
}

impl WindowTheme {
    fn as_str(&self) -> &str {
        match self {
            WindowTheme::Light => "light",
            WindowTheme::Dark => "dark",
            WindowTheme::System => "system",
        }
    }
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

    fn from_physical_position(position: PhysicalPosition<i32>, scale_factor: f64) -> Self {
        let position: LogicalPosition<f64> = position.to_logical(scale_factor);
        Self {
            x: position.x,
            y: position.y,
        }
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

impl<'a> From<&'a str> for UzWindowLevel {
    fn from(value: &'a str) -> Self {
        match value {
            "normal" => UzWindowLevel::Normal,
            "alwaysOnTop" => UzWindowLevel::AlwaysOnTop,
            "alwaysOnBottom" => UzWindowLevel::AlwaysOnBottom,
            _ => UzWindowLevel::Normal, // default to normal if unrecognized
        }
    }
}

// convert string to WindowTheme, defaulting to System if unrecognized

impl<'a> From<&'a str> for WindowTheme {
    fn from(value: &'a str) -> Self {
        match value {
            "light" => WindowTheme::Light,
            "dark" => WindowTheme::Dark,
            "system" => WindowTheme::System,
            _ => WindowTheme::System, // default to system if unrecognized
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

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            title: "uzumaki".to_string(),
            visible: true,
            resizable: true,
            decorations: true,
            transparent: false,
            maximized: false,
            minimized: false,
            fullscreen: false,
            window_level: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            position: None,
            theme: None,
            active: None,
            content_protected: false,
            closable: true,
            minimizable: true,
            maximizable: true,
        }
    }
}

impl WindowOptions {
    pub(crate) fn transparent(&self) -> bool {
        self.transparent
    }

    pub(crate) fn window_level(&self) -> WindowLevel {
        self.window_level
            .map(UzWindowLevel::to_winit)
            .unwrap_or(WindowLevel::Normal)
    }

    pub(crate) fn enabled_buttons(&self) -> WindowButtons {
        window_buttons(self.closable, self.minimizable, self.maximizable)
    }

    pub(crate) fn to_window_attributes(&self) -> WindowAttributes {
        let mut attributes = WindowAttributes::default()
            .with_title(self.title.clone())
            .with_inner_size(LogicalSize::new(self.width as f64, self.height as f64))
            .with_visible(self.visible)
            .with_resizable(self.resizable)
            .with_decorations(self.decorations)
            .with_transparent(self.transparent)
            .with_maximized(self.maximized)
            .with_window_level(self.window_level())
            .with_content_protected(self.content_protected)
            .with_enabled_buttons(self.enabled_buttons());

        if self.fullscreen {
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
        attributes
    }

    pub(crate) fn apply_post_create_state(&self, window: &WinitWindow) {
        if self.minimized {
            window.set_minimized(true);
        }
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

fn window_buttons(closable: bool, minimizable: bool, maximizable: bool) -> WindowButtons {
    let mut buttons = WindowButtons::empty();
    if closable {
        buttons |= WindowButtons::CLOSE;
    }
    if minimizable {
        buttons |= WindowButtons::MINIMIZE;
    }
    if maximizable {
        buttons |= WindowButtons::MAXIMIZE;
    }
    buttons
}

#[op2]
#[cppgc]
pub fn op_create_window(
    state: &mut OpState,
    #[serde] options: CreateWindowOptions,
) -> Result<CoreWindow, deno_error::JsErrorBox> {
    static NEXT_WINDOW_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
    let id = NEXT_WINDOW_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let options = WindowOptions::default().refined(options);

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
                content_protected: options.content_protected,
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

    fn has_window_button(&self, state: &OpState, button: WindowButtons) -> Option<bool> {
        self.with_window_entry(state, |entry| entry.enabled_buttons.contains(button))
    }

    fn set_window_button_state(
        &self,
        state: &OpState,
        button: WindowButtons,
        enabled: bool,
    ) -> bool {
        self.with_window_entry_mut(state, |entry| {
            if enabled {
                entry.enabled_buttons |= button;
            } else {
                entry.enabled_buttons &= !button;
            }
            if let Some(handle) = entry.handle.as_ref() {
                handle
                    .winit_window
                    .set_enabled_buttons(entry.enabled_buttons);
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
    #[setter]
    pub fn title(&self, state: &OpState, #[string] title: String) -> bool {
        self.update_winit_window(state, |window| window.set_title(&title))
    }

    #[getter]
    pub fn visible(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window_option(state, |window| window.is_visible())
    }

    #[fast]
    #[setter]
    pub fn visible(&self, state: &OpState, visible: bool) -> bool {
        self.update_winit_window(state, |window| window.set_visible(visible))
    }

    #[getter]
    pub fn transparent(&self, state: &OpState) -> Option<bool> {
        self.with_window_entry(state, |entry| entry.transparent)
    }

    #[fast]
    #[setter]
    pub fn transparent(&self, state: &OpState, transparent: bool) -> bool {
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
    #[setter]
    pub fn resizable(&self, state: &OpState, resizable: bool) -> bool {
        self.update_winit_window(state, |window| window.set_resizable(resizable))
    }

    #[getter]
    pub fn decorations(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window(state, |window| window.is_decorated())
    }

    #[fast]
    #[setter]
    pub fn decorations(&self, state: &OpState, decorations: bool) -> bool {
        self.update_winit_window(state, |window| window.set_decorations(decorations))
    }

    #[getter]
    pub fn maximized(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window(state, |window| window.is_maximized())
    }

    #[fast]
    #[setter]
    pub fn maximized(&self, state: &OpState, maximized: bool) -> bool {
        self.update_winit_window(state, |window| window.set_maximized(maximized))
    }

    #[getter]
    pub fn minimized(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window_option(state, |window| window.is_minimized())
    }

    #[fast]
    #[setter]
    pub fn minimized(&self, state: &OpState, minimized: bool) -> bool {
        self.update_winit_window(state, |window| window.set_minimized(minimized))
    }

    #[getter]
    pub fn fullscreen(&self, state: &OpState) -> Option<bool> {
        self.with_winit_window(state, |window| window.fullscreen().is_some())
    }

    #[fast]
    #[setter]
    pub fn fullscreen(&self, state: &OpState, fullscreen: bool) -> bool {
        self.update_winit_window(state, |window| {
            let target = fullscreen.then_some(Fullscreen::Borderless(None));
            window.set_fullscreen(target);
        })
    }

    // todo change to int
    #[getter]
    #[string]
    pub fn windowLevel(&self, state: &OpState) -> Option<String> {
        self.with_window_entry(state, |entry| UzWindowLevel::from_winit(entry.window_level))
            .map(|level| level.as_str().into())
    }

    #[fast]
    #[setter]
    pub fn windowLevel(&self, state: &OpState, #[string] level: &str) -> bool {
        let level = UzWindowLevel::from(level);
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
            window.outer_position().ok().map(|position| {
                WindowPosition::from_physical_position(position, window.scale_factor())
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
    #[string]
    pub fn theme(&self, state: &OpState) -> Option<String> {
        self.with_winit_window_option(state, |window| window.theme().map(WindowTheme::from_winit))
            .map(|theme| theme.as_str().into())
    }

    // todo use int
    #[fast]
    #[setter]
    pub fn theme(&self, state: &OpState, #[string] theme: &str) -> bool {
        let theme = WindowTheme::from(theme);
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
    pub fn closable(&self, state: &OpState) -> Option<bool> {
        self.has_window_button(state, WindowButtons::CLOSE)
    }

    #[fast]
    #[setter]
    pub fn closable(&self, state: &OpState, closable: bool) -> bool {
        self.set_window_button_state(state, WindowButtons::CLOSE, closable)
    }

    #[getter]
    pub fn minimizable(&self, state: &OpState) -> Option<bool> {
        self.has_window_button(state, WindowButtons::MINIMIZE)
    }

    #[fast]
    #[setter]
    pub fn minimizable(&self, state: &OpState, minimizable: bool) -> bool {
        self.set_window_button_state(state, WindowButtons::MINIMIZE, minimizable)
    }

    #[getter]
    pub fn maximizable(&self, state: &OpState) -> Option<bool> {
        self.has_window_button(state, WindowButtons::MAXIMIZE)
    }

    #[fast]
    #[setter]
    pub fn maximizable(&self, state: &OpState, maximizable: bool) -> bool {
        self.set_window_button_state(state, WindowButtons::MAXIMIZE, maximizable)
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
    use super::{WindowOptions, WindowPosition, WindowTheme};
    use winit::dpi::PhysicalPosition;
    use winit::window::{Fullscreen, Theme};

    fn base_options() -> WindowOptions {
        WindowOptions {
            width: 800,
            height: 600,
            title: "demo".to_string(),
            ..WindowOptions::default()
        }
    }

    #[test]
    fn create_options_map_common_attributes() {
        let mut options = base_options();
        options.visible = false;
        options.resizable = false;
        options.decorations = false;
        options.transparent = true;
        options.maximized = true;
        options.fullscreen = true;
        options.theme = Some(WindowTheme::Dark);
        options.active = Some(true);
        options.content_protected = true;

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
    fn physical_position_is_reported_as_logical_position() {
        let position = WindowPosition::from_physical_position(PhysicalPosition::new(300, 150), 1.5);

        assert_eq!(position, WindowPosition { x: 200.0, y: 100.0 });
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
    fn explicit_window_level_maps_to_window_attributes() {
        let mut options = base_options();
        options.window_level = Some(super::UzWindowLevel::AlwaysOnBottom);

        assert_eq!(
            options.window_level(),
            winit::window::WindowLevel::AlwaysOnBottom
        );
        assert_eq!(
            options.to_window_attributes().window_level,
            winit::window::WindowLevel::AlwaysOnBottom
        );
    }

    #[test]
    fn flat_button_options_map_to_enabled_buttons() {
        let mut options = base_options();
        options.closable = false;
        options.minimizable = true;
        options.maximizable = false;

        let buttons = options.enabled_buttons();

        assert!(!buttons.contains(winit::window::WindowButtons::CLOSE));
        assert!(buttons.contains(winit::window::WindowButtons::MINIMIZE));
        assert!(!buttons.contains(winit::window::WindowButtons::MAXIMIZE));
        assert_eq!(options.to_window_attributes().enabled_buttons, buttons);
    }

    #[test]
    fn minimized_is_post_create_state() {
        let mut options = base_options();
        options.minimized = true;

        assert!(options.minimized);
    }
}
