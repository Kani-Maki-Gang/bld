use leptos::{html::Dialog, *};

#[derive(Copy, Clone)]
pub struct AppDialog(pub NodeRef<Dialog>);

#[derive(Copy, Clone)]
pub struct AppDialogContent(pub RwSignal<Option<View>>);

#[derive(Copy, Clone)]
pub struct RefreshCronJobs(pub RwSignal<()>);

impl RefreshCronJobs {
    pub fn set(&self) {
        self.0.set(());
    }
}

#[derive(Copy, Clone)]
pub struct RefreshHistory(pub RwSignal<()>);

impl RefreshHistory {
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
