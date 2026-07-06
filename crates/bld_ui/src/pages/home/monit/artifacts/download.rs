use crate::{
    api,
    components::button::IconButton,
    context::{AppDialog, AppDialogContent},
    error::ErrorDialog,
};
use anyhow::{Result, anyhow};
use leptos::{leptos_dom::logging, *};
use wasm_bindgen::JsCast;
use web_sys::{HtmlAnchorElement, Url, window};

type DownloadActionArgs = (i32, String, Option<AppDialog>, Option<AppDialogContent>);

fn save_bytes_as_file(bytes: Vec<u8>, filename: &str) -> Result<()> {
    let array = js_sys::Uint8Array::from(bytes.as_slice());
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

    let body = document.body().ok_or_else(|| anyhow!("document body not found"))?;
    body.append_child(&anchor)
        .map_err(|e| anyhow!("unable to append anchor element: {e:?}"))?;
    anchor.click();
    body.remove_child(&anchor)
        .map_err(|e| anyhow!("unable to remove anchor element: {e:?}"))?;

    Url::revoke_object_url(&url).map_err(|e| anyhow!("unable to revoke object url: {e:?}"))?;

    Ok(())
}

#[component]
pub fn ArtifactDownloadButton(#[prop(into)] id: i32, #[prop(into)] name: String) -> impl IntoView {
    let app_dialog = use_context::<AppDialog>();
    let app_dialog_content = use_context::<AppDialogContent>();

    let download_action = create_action(|args: &DownloadActionArgs| {
        let (id, name, app_dialog, app_dialog_content) = args.clone();
        async move {
            let result = async {
                let bytes = api::artifact_download(id).await?;
                save_bytes_as_file(bytes, &format!("{name}.tar.gz"))
            }
            .await;

            if let Err(e) = result {
                let Some(AppDialog(dialog)) = app_dialog else {
                    logging::console_error("App dialog context not found");
                    return;
                };
                let Some(AppDialogContent(content)) = app_dialog_content else {
                    logging::console_error("App dialog context not found");
                    return;
                };
                content.set(Some(
                    view! { <ErrorDialog dialog=dialog error=move || e.to_string() /> },
                ));
                let _ = dialog.get().map(|x| x.show_modal());
            }
        }
    });

    view! {
        <IconButton
            icon="iconoir-download"
            ghost=true
            on:click=move |_| {
                download_action.dispatch((id, name.clone(), app_dialog, app_dialog_content));
            }
        />
    }
}
