use crate::components::card::Card;
use leptos::*;
use chart_js_rs::{bar::Bar, Dataset, NoAnnotations, XYDataset};

#[component]
pub fn DashboardRunsPerMonth() -> impl IntoView {
    let months = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let chart = Bar::<NoAnnotations> {
        data: Dataset {
            labels: Some(months.iter().map(|x| x.into()).collect()),
            datasets: vec![
                XYDataset{
                    label: Some("Runs".into()),
                    data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
                    ..Default::default()
                }
            ]
        }
        ..Default::default()
    };
    view! {
        <Card>
            <div class="flex flex-col px-8 py-12">
                <div class="text-2xl">"Runs per month"</div>
                <div class="text-gray-400 mb-8">"Overview"</div>
            </div>
        </Card>
    }
}
