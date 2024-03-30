use crate::components::card::Card;
use leptos::*;
use leptos_chartistry::*;

struct MyData {
    pub month: f64,
    pub runs: f64,
}

#[component]
pub fn DashboardRunsPerMonth() -> impl IntoView {
    let series = Series::new(|data: &MyData| data.month)
        .with_min_y(0.0)
        .with_max_y(150.0)
        .bar(|data: &MyData| data.runs);

    let (data, _set_data) = create_signal(vec![
        MyData {
            month: 1.0,
            runs: 10.0,
        },
        MyData {
            month: 2.0,
            runs: 20.0,
        },
        MyData {
            month: 3.0,
            runs: 30.0,
        },
        MyData {
            month: 4.0,
            runs: 40.0,
        },
        MyData {
            month: 5.0,
            runs: 50.0,
        },
        MyData {
            month: 6.0,
            runs: 60.0,
        },
        MyData {
            month: 7.0,
            runs: 70.0,
        },
        MyData {
            month: 8.0,
            runs: 80.0,
        },
        MyData {
            month: 9.0,
            runs: 90.0,
        },
        MyData {
            month: 10.0,
            runs: 100.0,
        },
        MyData {
            month: 11.0,
            runs: 110.0,
        },
        MyData {
            month: 12.0,
            runs: 120.0,
        },
    ]);

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12">
                <div class="text-2xl">"Total runs per month"</div>
                <div class="text-gray-400 mb-8">"Aggregated data for all pipelines on the server"</div>
                <div class="grow">
                    <Chart
                        aspect_ratio=AspectRatio::from_inner_ratio(1150.0, 500.0)
                        series=series
                        data=data
                        left=TickLabels::aligned_floats()
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
