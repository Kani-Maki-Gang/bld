use crate::{api, components::card::Card, error::SmallError};
use bld_models::dtos::PipelineRunsPerMonthKpi;
use html::Div;
use leptos::*;
use leptos_chartistry::*;
use leptos_use::{use_element_size, UseElementSizeReturn};

#[component]
pub fn DashboardRunsPerMonth() -> impl IntoView {
    let card = create_node_ref::<Div>();
    let UseElementSizeReturn { width, .. } = use_element_size(card);
    let series = Series::new(|data: &PipelineRunsPerMonthKpi| data.month)
        .with_min_y(0.0)
        .with_max_y(150.0)
        .bar(|data: &PipelineRunsPerMonthKpi| data.count);

    let data = create_resource(
        || (),
        |_| async move {
            api::pipeline_runs_per_month()
                .await
                .map_err(|e| e.to_string())
        },
    );

    let chart_data = move || {
        if let Some(Ok(data)) = data.get() {
            series.max_y.set(
                data.iter()
                    .map(|x| x.count as i64)
                    .max()
                    .map(|x| x as f64),
            );
            data
        } else {
            vec![]
        }
    };

    let aspect_ratio = move || {
        AspectRatio::from_inner_ratio(width.get() - 100.0, 500.0)
    };

    view! {
        <Card>
            <div node_ref=card class="flex flex-col px-8 py-12 gap-4">
                <div class="flex flex-col">
                    <div class="text-2xl">"Total runs per month"</div>
                    <div class="text-gray-400 mb-8">
                        "Aggregated data for all pipelines on the server"
                    </div>
                </div>
                <Show when=move || matches!(data.get(), Some(Err(_))) fallback=|| view! {}>
                    <SmallError error=move || data.get().unwrap().unwrap_err()/>
                </Show>
                <div class="grow">
                    <Chart
                        aspect_ratio=Signal::from(move || aspect_ratio())
                        series=series
                        data=move || chart_data()
                        left=TickLabels::aligned_floats()
                        bottom=TickLabels::aligned_floats()
                        inner=[
                            AxisMarker::left_edge().into_inner(),
                            AxisMarker::bottom_edge().into_inner(),
                        ]
                    />
                </div>
            </div>
        </Card>
    }
}
