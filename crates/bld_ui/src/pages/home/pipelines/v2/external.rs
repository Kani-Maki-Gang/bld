use crate::components::{badge::Badge, card::Card, list::List};
use bld_runner::external::v2::External;
use leptos::{leptos_dom::logging, *};

#[component]
pub fn PipelineExternalV2(#[prop(into)] external: Signal<Vec<External>>) -> impl IntoView {
    let external = move || {
        external
            .get()
            .into_iter()
            .map(|x| {
                serde_yaml::to_string(&x)
                    .map_err(|e| logging::console_error(&format!("{:?}", e)))
                    .unwrap_or_default()
            })
            .collect::<Vec<String>>()
    };

    view! {
        <Card class="min-h-full">
            <div class="flex flex-col px-8 py-12 gap-y-4">
                <div class="flex flex-col">
                    <div class="text-xl">
                        "External"
                    </div>
                    <div class="text-gray-400">
                        "The external pipelines local or server used by this pipeline."
                    </div>
                </div>
                <Show
                    when=move || !external().is_empty()
                    fallback= || view! {
                        <div class="grid justify-items-center">
                            <Badge>"No external pipelines configured."</Badge>
                        </div>
                    }>
                    <List>
                        <For
                            each=move || external().into_iter().enumerate()
                            key=|(i, _)| *i
                            let:child>
                            <pre class="text-sm text-gray-200 p-4 rounded-lg bg-slate-800">
                                {child.1}
                            </pre>
                        </For>
                    </List>
                </Show>
            </div>
        </Card>
    }
}
