use leptos::*;

#[component]
pub fn MonitLogs(#[prop(into)] history: Signal<Vec<String>>) -> impl IntoView {
    view! {
        <div class="px-6 py-5 grow">
            <div class="bg-zinc-950 border border-zinc-800 rounded-xl p-5 font-mono text-xs text-emerald-400 leading-relaxed min-h-[300px] overflow-auto">
                <For
                    each=move || history.get().into_iter().enumerate()
                    key=|(index, _)| *index
                    let:child
                >
                    <pre>{child.1}</pre>
                </For>
            </div>
        </div>
    }
}
