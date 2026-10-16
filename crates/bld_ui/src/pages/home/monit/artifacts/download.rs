use crate::{
    api, browser::save_bytes_as_file, components::button::IconButton, context::use_app_dialog,
};
use leptos::*;

#[component]
pub fn ArtifactDownloadButton(#[prop()] id: String, #[prop()] name: String) -> impl IntoView {
    let dialog = use_app_dialog();
    let download = create_action(move |(id, name): &(String, String)| {
        let (id, name) = (id.clone(), name.clone());
        async move {
            let result = async {
                let bytes = api::artifact_download(id).await?;
                save_bytes_as_file(&bytes, &format!("{name}.tar.gz"))
            }
            .await;
            if let Err(e) = result {
                dialog.error(e.to_string());
            }
        }
    });

    view! {
        <IconButton
            icon="iconoir-download"
            title="Download artifact"
            on:click=move |_| download.dispatch((id.clone(), name.clone()))
        />
    }
}
