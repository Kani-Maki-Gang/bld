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
        <div class="overflow-x-auto rounded-lg border-2 border-slate-600">
            <table class="min-w-full bg-slate-700 text-sm">
                <thead>
                    <tr>
                        <For each=move || headers.get() key=|state| state.clone() let:child>
                            <th class="border-2 border-slate-600 whitespace-nowrap px-8 py-4 font-bold text-lg text-left">{child}</th>
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
                                        <td class="border border-slate-600 whitespace-nowrap px-8 py-4 text-left">{c}</td>
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
