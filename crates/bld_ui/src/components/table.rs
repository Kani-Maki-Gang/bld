use leptos::*;

#[derive(Debug, Clone)]
pub struct TableRow {
    pub columns: Vec<View>,
}

#[component]
pub fn Table(
    #[prop(into)] headers: Signal<Vec<View>>,
    #[prop(into)] rows: Signal<Vec<TableRow>>,
) -> impl IntoView {
    view! {
        <div class="overflow-x-auto">
            <table class="min-w-full bg-slate-700 text-sm">
                <thead>
                    <tr>
                        {move || headers
                            .get()
                            .into_iter()
                            .map(|h| view! {
                                <th class="border border-b-4 border-slate-600 whitespace-nowrap p-4 font-bold text-left">{h}</th>
                            }.into_view())
                            .collect::<View>()
                        }
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

#[component]
pub fn DataTable(children: Children) -> impl IntoView {
    view! {
        <div class="overflow-x-auto">
            <table class="min-w-full bg-slate-700 text-sm">
                {children()}
            </table>
        </div>
    }
}

#[component]
pub fn Headers(children: Children) -> impl IntoView {
    view! {
        <thead>
            <tr>
                {children()}
            </tr>
        </thead>
    }
}

#[component]
pub fn Header(children: Children) -> impl IntoView {
    view! {
        <th class="border border-b-4 border-slate-600 whitespace-nowrap p-4 font-bold text-left">
            {children()}
        </th>
    }
}

#[component]
pub fn Body(children: Children) -> impl IntoView {
    view! {
        <tbody>
            {children()}
        </tbody>
    }
}

#[component]
pub fn Row(children: Children) -> impl IntoView {
    view! {
        <tr>
            {children()}
        </tr>
    }
}

#[component]
pub fn Cell(children: Children) -> impl IntoView {
    view! {
        <td class="border border-slate-600 whitespace-nowrap p-4 text-left">
            {children()}
        </td>
    }
}
