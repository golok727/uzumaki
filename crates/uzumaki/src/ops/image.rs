use deno_core::{JsBuffer, OpState, op2};
use deno_error::JsErrorBox;
use image::GenericImageView;
use vello::peniko::{Blob, ImageAlphaType, ImageData, ImageFormat};

use crate::app::{SharedAppState, with_state};
use crate::element::UzNodeId;

fn window_not_found() -> JsErrorBox {
    JsErrorBox::new("WindowNotFound", "window not found")
}

fn invalid_image_data(error: impl std::fmt::Display) -> JsErrorBox {
    JsErrorBox::new("InvalidImageData", error.to_string())
}

#[op2]
pub fn op_set_encoded_image_data(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[buffer] data: JsBuffer,
) -> Result<(), JsErrorBox> {
    let nid = node_id as UzNodeId;
    let decoded =
        image::load_from_memory(&data).map_err(|err| invalid_image_data(err.to_string()))?;

    let (width, height) = decoded.dimensions();

    let rgba = decoded.to_rgba8();
    let image = ImageData {
        data: Blob::from(rgba.into_raw()),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::Alpha,
        width,
        height,
    };

    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return Err(window_not_found());
        };
        entry.dom.set_image_data(nid, width, height, image);
        Ok(())
    })
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
        entry.dom.clear_image_data(nid);
        Ok(())
    })
}
