use super::colors::Colors;
use leptos::*;

pub fn get_button_color_classes(color: Option<Colors>) -> &'static str {
    let color = color.unwrap_or_else(|| Colors::Indigo);
    match color {
        Colors::Slate => "bg-slate-600 hover:bg-slate-700 focus:bg-slate-700",
        Colors::Gray => "bg-gray-600 hover:bg-gray-700 focus:bg-gray-700",
        Colors::Zinc => "bg-zinc-600 hover:bg-zinc-700 focus:bg-zinc-700",
        Colors::Neutral => "bg-neutral-600 hover:bg-neutral-700 focus:bg-neutral-700",
        Colors::Stone => "bg-stone-600 hover:bg-stone-700 focus:bg-stone-700",
        Colors::Red => "bg-red-600 hover:bg-red-700 focus:bg-red-700",
        Colors::Orange => "bg-orange-600 hover:bg-orange-700 focus:bg-orange-700",
        Colors::Amber => "bg-amber-600 hover:bg-amber-700 focus:bg-amber-700",
        Colors::Yellow => "bg-yellow-600 hover:bg-yellow-700 focus:bg-yellow-700",
        Colors::Lime => "bg-lime-600 hover:bg-lime-700 focus:bg-lime-700",
        Colors::Green => "bg-green-600 hover:bg-green-700 focus:bg-green-700",
        Colors::Emerald => "bg-emerald-600 hover:bg-emerald-700 focus:bg-emerald-700",
        Colors::Teal => "bg-teal-600 hover:bg-teal-700 focus:bg-teal-700",
        Colors::Cyan => "bg-cyan-600 hover:bg-cyan-700 focus:bg-cyan-700",
        Colors::Sky => "bg-sky-600 hover:bg-sky-700 focus:bg-sky-700",
        Colors::Blue => "bg-blue-600 hover:bg-blue-700 focus:bg-blue-700",
        Colors::Indigo => "bg-indigo-600 hover:bg-indigo-700 focus:bg-indigo-700",
        Colors::Violet => "bg-violet-600 hover:bg-violet-700 focus:bg-violet-700",
        Colors::Purple => "bg-purple-600 hover:bg-purple-700 focus:bg-purple-700",
        Colors::Fuchsia => "bg-fuchsia-600 hover:bg-fuchsia-700 focus:bg-fuchsia-700",
        Colors::Pink => "bg-pink-600 hover:bg-pink-700 focus:bg-pink-700",
        Colors::Rose => "bg-rose-600 hover:bg-rose-700 focus:bg-rose-700",
    }
}

#[component]
pub fn Button(
    #[prop(into, optional)] color: Option<Colors>,
    #[prop(into, optional)] class: String,
    children: Children,
) -> impl IntoView {
    let color = get_button_color_classes(color);
    let class = format!("h-[40px] w-full rounded-lg p-2 focus:outline-none {color} {class}");
    view! { <button class=class>{children()}</button> }
}

#[component]
pub fn IconButton(
    #[prop(into, optional)] color: Option<Colors>,
    #[prop(into, optional)] class: String,
    #[prop(into)] icon: String,
) -> impl IntoView {
    let color = get_button_color_classes(color);
    let class =
        format!("h-[40px] w-[40px] text-xl rounded-lg p-2 focus:outline-none {color} {class}");
    view! {
        <button class=class>
            <i class=icon></i>
        </button>
    }
}
