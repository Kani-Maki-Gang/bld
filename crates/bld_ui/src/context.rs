use leptos::{html::Dialog, *};

#[derive(Copy, Clone)]
pub struct AppDialog(pub NodeRef<Dialog>);

#[derive(Copy, Clone)]
pub struct AppDialogContent(pub RwSignal<Option<View>>);

#[derive(Copy, Clone)]
pub enum PipelineView {
    UI,
    RawFile
}

#[derive(Copy, Clone)]
pub struct PipelineSelectedView(pub RwSignal<PipelineView>);

impl PipelineSelectedView {
    pub fn get(&self) -> PipelineView {
        self.0.get()
    }

    pub fn set(&self, view: PipelineView) {
        self.0.set(view);
    }
}

#[derive(Copy, Clone)]
pub struct RefreshCronJobs(pub RwSignal<()>);

impl RefreshCronJobs {
    pub fn get(&self) -> () {
        self.0.get()
    }

    pub fn set(&self) {
        self.0.set(());
    }
}

#[derive(Copy, Clone)]
pub struct RefreshHistory(pub RwSignal<()>);

impl RefreshHistory {
    pub fn get(&self) -> () {
        self.0.get()
    }

    pub fn set(&self) {
        self.0.set(());
    }
}
