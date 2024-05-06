use crate::components::{
    badge::Badge,
    card::Card,
    list::{List, ListItem},
};
use bld_runner::external::v2::External;
use leptos::{leptos_dom::logging, *};

#[component]
pub fn PipelineExternalV2(#[prop(into)] external: Signal<Vec<External>>) -> impl IntoView {
    let external = move || {
        external
            .get()
            .into_iter()
            .map(|x| {
                let mut item = ListItem::default();
                item.icon = "iconoir-minus".to_string();
                let content = serde_yaml::to_string(&x)
                    .map_err(|e| logging::console_error(&format!("{:?}", e)))
                    .unwrap_or_default();
                item.content = Some(
                    view! {
                        <pre class="text-sm text-gray-200">
                            {content}
                        </pre>
                    }
                    .into_view(),
                );
                item
            })
            .collect::<Vec<ListItem>>()
    };

    view! {
        <Card>
            <div class="flex flex-col px-8 py-12 gap-y-4 min-h-96 max-h-[600]px">
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
                    <List items=external />
                </Show>
            </div>
        </Card>
    }
}
