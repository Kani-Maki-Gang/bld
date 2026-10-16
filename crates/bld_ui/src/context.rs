use crate::error::ErrorDialog;
use leptos::{html::Dialog, *};

#[derive(Copy, Clone)]
pub struct AppDialog {
    pub node: NodeRef<Dialog>,
    pub content: RwSignal<Option<View>>,
    pub owner: Option<Owner>,
}

impl AppDialog {
    pub fn open<V: IntoView>(&self, content: impl FnOnce() -> V + 'static) {
        let view = match self.owner {
            Some(owner) => with_owner(owner, move || content().into_view()),
            None => content().into_view(),
        };
        self.content.set(Some(view));
        let _ = self.node.get_untracked().map(|x| x.show_modal());
    }

    pub fn close(&self) {
        let _ = self.node.get_untracked().map(|x| x.close());
    }

    pub fn error(&self, error: String) {
        self.open(move || view! { <ErrorDialog error=error /> });
    }
}

pub fn use_app_dialog() -> AppDialog {
    let mut dialog = expect_context::<AppDialog>();
    dialog.owner = Owner::current();
    dialog
}

#[derive(Copy, Clone)]
pub struct RefreshCronJobs(pub RwSignal<()>);

impl RefreshCronJobs {
    pub fn set(&self) {
        self.0.set(());
    }
}

#[derive(Copy, Clone)]
pub struct RefreshArtifacts(pub RwSignal<()>);

impl RefreshArtifacts {
    pub fn set(&self) {
        self.0.set(());
    }
}

#[derive(Copy, Clone)]
pub struct RefreshPipelines(pub RwSignal<()>);

impl RefreshPipelines {
    pub fn get(&self) {
        self.0.get();
    }

    pub fn set(&self) {
        self.0.set(());
    }
}
