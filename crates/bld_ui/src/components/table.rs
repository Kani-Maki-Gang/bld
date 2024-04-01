use leptos::*;

#[derive(Debug, Clone)]
pub struct TableRow {
    pub columns: Vec<String>,
}

#[component]
pub fn Table(
    #[prop()] headers: Signal<Vec<String>>,
    #[prop()] rows: Signal<Vec<TableRow>>,
) -> impl IntoView {
    view! {
        <div class="overflow-x-auto">
            <table class="min-w-full bg-slate-700 text-sm">
                <thead>
                    <tr>
                        <For each=move || headers.get() key=|state| state.clone() let:child>
                            <th class="border border-b-4 border-slate-600 whitespace-nowrap p-4 font-bold text-left">{child}</th>
                        </For>
                    </tr>
                </thead>
                <tbody>
                    {move || rows
                        .get()
                        .into_iter()
                        .map(|row| view! {
                            <tr>
                                {move || row
                                    .columns
                                    .iter()
                                    .map(|c| view! {
                                        <td class="border border-slate-600 whitespace-nowrap p-4 text-left">{c}</td>
                                    }.into_view())
                                    .collect::<View>()
                                }
                            </tr>
                        }.into_view())
                        .collect::<View>()
                    }
                </tbody>
            </table>
        </div>
    }
}
