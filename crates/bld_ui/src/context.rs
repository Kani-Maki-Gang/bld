use leptos::{html::Dialog, *};

#[derive(Clone)]
pub struct AppDialog(pub NodeRef<Dialog>);

#[derive(Clone)]
pub struct AppDialogContent(pub RwSignal<Option<View>>);

#[derive(Clone)]
pub struct RefreshCronJobs(pub RwSignal<()>);

#[derive(Clone)]
pub struct RefreshHistory(pub RwSignal<()>);
