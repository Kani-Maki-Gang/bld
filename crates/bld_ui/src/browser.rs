use anyhow::{Result, anyhow};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlAnchorElement, Url, window};

pub fn save_bytes_as_file(bytes: &[u8], filename: &str) -> Result<()> {
    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::of1(&array);
    let blob = web_sys::Blob::new_with_u8_array_sequence(&parts)
        .map_err(|e| anyhow!("unable to create blob: {e:?}"))?;
    let url = Url::create_object_url_with_blob(&blob)
        .map_err(|e| anyhow!("unable to create object url: {e:?}"))?;

    let document = window()
        .ok_or_else(|| anyhow!("window not found"))?
        .document()
        .ok_or_else(|| anyhow!("document not found"))?;

    let anchor: HtmlAnchorElement = document
        .create_element("a")
        .map_err(|e| anyhow!("unable to create anchor element: {e:?}"))?
        .dyn_into()
        .map_err(|_| anyhow!("unable to cast to anchor element"))?;
    anchor.set_href(&url);
    anchor.set_download(filename);

    let body = document
        .body()
        .ok_or_else(|| anyhow!("document body not found"))?;
    body.append_child(&anchor)
        .map_err(|e| anyhow!("unable to append anchor element: {e:?}"))?;
    anchor.click();
    body.remove_child(&anchor)
        .map_err(|e| anyhow!("unable to remove anchor element: {e:?}"))?;

    Url::revoke_object_url(&url).map_err(|e| anyhow!("unable to revoke object url: {e:?}"))?;

    Ok(())
}

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let navigator = window()
        .ok_or_else(|| anyhow!("window not found"))?
        .navigator();
    let clipboard = js_sys::Reflect::get(&navigator, &JsValue::from_str("clipboard"))
        .map_err(|e| anyhow!("clipboard not available: {e:?}"))?;
    let write_text: js_sys::Function =
        js_sys::Reflect::get(&clipboard, &JsValue::from_str("writeText"))
            .map_err(|e| anyhow!("clipboard not available: {e:?}"))?
            .dyn_into()
            .map_err(|_| anyhow!("clipboard writeText is not a function"))?;
    write_text
        .call1(&clipboard, &JsValue::from_str(text))
        .map_err(|e| anyhow!("unable to copy to clipboard: {e:?}"))?;
    Ok(())
}
