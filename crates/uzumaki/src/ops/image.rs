use std::sync::Arc;

use deno_core::{JsBuffer, OpState, op2};
use deno_error::JsErrorBox;
use image::GenericImageView;

use crate::app::{SharedAppState, with_state};
use crate::element::{ImageData, RasterImageData, UzNodeId};

fn window_not_found() -> JsErrorBox {
    JsErrorBox::new("WindowNotFound", "window not found")
}

fn invalid_image_data(error: impl std::fmt::Display) -> JsErrorBox {
    JsErrorBox::new("InvalidImageData", error.to_string())
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let head = &bytes[i..bytes.len().min(i + 512)];
    let s = std::str::from_utf8(head).unwrap_or("");
    s.starts_with("<?xml") || s.starts_with("<svg") || s.contains("<svg")
}

fn decode(data: &[u8]) -> Result<ImageData, JsErrorBox> {
    if looks_like_svg(data) {
        let opts = usvg::Options::default();
        let tree = usvg::Tree::from_data(data, &opts).map_err(invalid_image_data)?;
        let text = std::str::from_utf8(data).unwrap_or("");
        return Ok(ImageData::Svg {
            tree: Arc::new(tree),
            uses_current_color: text.contains("currentColor") || text.contains("currentcolor"),
        });
    }
    let decoded = image::load_from_memory(data).map_err(invalid_image_data)?;
    let (width, height) = decoded.dimensions();
    let rgba = decoded.to_rgba8();
    Ok(ImageData::Raster(RasterImageData::new(
        width,
        height,
        Arc::new(rgba.into_raw()),
    )))
}

#[op2]
pub fn op_set_encoded_image_data(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] cache_key: String,
    #[buffer] data: JsBuffer,
) -> Result<(), JsErrorBox> {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();

    let cached = with_state(&app_state, |s| s.image_cache.get(&cache_key).cloned());
    let image = match cached {
        Some(img) => img,
        None => {
            let decoded = decode(&data)?;
            with_state(&app_state, |s| {
                s.image_cache.insert(cache_key.clone(), decoded.clone());
            });
            decoded
        }
    };

    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(window_not_found());
        };
        entry.dom.set_image_data(nid, image);
        Ok(())
    })
}

#[op2(fast)]
pub fn op_apply_cached_image(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] cache_key: String,
) -> bool {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();

    let cached = with_state(&app_state, |s| s.image_cache.get(&cache_key).cloned());
    let Some(image) = cached else {
        return false;
    };

    with_state(&app_state, |s| {
        if let Some(entry) = s.windows.get_mut(&window_id) {
            entry.dom.set_image_data(nid, image);
        }
    });
    true
}

#[op2(fast)]
pub fn op_clear_image_data(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
) -> Result<(), JsErrorBox> {
    let nid = node_id as UzNodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(window_not_found());
        };
        entry.dom.set_image_data(nid, ImageData::None);
        Ok(())
    })
}
