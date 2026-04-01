use super::colors::Colors;
use leptos::*;

pub fn get_button_color_classes(color: Option<Colors>) -> &'static str {
    let color = color.unwrap_or_else(|| Colors::Violet);
    match color {
        Colors::Slate => "bg-slate-700 hover:bg-slate-600 focus:bg-slate-600",
        Colors::Gray => "bg-gray-700 hover:bg-gray-600 focus:bg-gray-600",
        Colors::Zinc => "bg-zinc-700 hover:bg-zinc-600 focus:bg-zinc-600",
        Colors::Neutral => "bg-neutral-700 hover:bg-neutral-600 focus:bg-neutral-600",
        Colors::Stone => "bg-stone-700 hover:bg-stone-600 focus:bg-stone-600",
        Colors::Red => "bg-red-600 hover:bg-red-500 focus:bg-red-500",
        Colors::Orange => "bg-orange-600 hover:bg-orange-500 focus:bg-orange-500",
        Colors::Amber => "bg-amber-600 hover:bg-amber-500 focus:bg-amber-500",
        Colors::Yellow => "bg-yellow-600 hover:bg-yellow-500 focus:bg-yellow-500",
        Colors::Lime => "bg-lime-600 hover:bg-lime-500 focus:bg-lime-500",
        Colors::Green => "bg-green-600 hover:bg-green-500 focus:bg-green-500",
        Colors::Emerald => "bg-emerald-600 hover:bg-emerald-500 focus:bg-emerald-500",
        Colors::Teal => "bg-teal-600 hover:bg-teal-500 focus:bg-teal-500",
        Colors::Cyan => "bg-cyan-600 hover:bg-cyan-500 focus:bg-cyan-500",
        Colors::Sky => "bg-sky-600 hover:bg-sky-500 focus:bg-sky-500",
        Colors::Blue => "bg-blue-600 hover:bg-blue-500 focus:bg-blue-500",
        Colors::Indigo => "bg-indigo-600 hover:bg-indigo-500 focus:bg-indigo-500",
        Colors::Violet => "bg-violet-600 hover:bg-violet-500 focus:bg-violet-500",
        Colors::Purple => "bg-purple-600 hover:bg-purple-500 focus:bg-purple-500",
        Colors::Fuchsia => "bg-fuchsia-600 hover:bg-fuchsia-500 focus:bg-fuchsia-500",
        Colors::Pink => "bg-pink-600 hover:bg-pink-500 focus:bg-pink-500",
        Colors::Rose => "bg-rose-600 hover:bg-rose-500 focus:bg-rose-500",
    }
}

pub fn get_ghost_icon_button_color_classes(color: Option<Colors>) -> &'static str {
    match color.unwrap_or(Colors::Zinc) {
        Colors::Red | Colors::Rose => "text-zinc-500 hover:text-red-400 hover:bg-red-500/10",
        Colors::Green | Colors::Emerald | Colors::Teal => {
            "text-zinc-500 hover:text-emerald-400 hover:bg-emerald-500/10"
        }
        Colors::Violet | Colors::Purple | Colors::Indigo => {
            "text-zinc-500 hover:text-violet-400 hover:bg-violet-500/10"
        }
        Colors::Blue | Colors::Sky | Colors::Cyan => {
            "text-zinc-500 hover:text-sky-400 hover:bg-sky-500/10"
        }
        _ => "text-zinc-500 hover:text-zinc-100 hover:bg-zinc-800",
    }
}

#[component]
pub fn Button(
    #[prop(into, optional)] color: Option<Colors>,
    #[prop(into, optional)] ghost: bool,
    #[prop(into, optional)] class: String,
    children: Children,
) -> impl IntoView {
    let class = if ghost {
        format!(
            "h-[38px] w-full rounded-lg px-4 py-2 text-sm font-medium bg-transparent border border-zinc-700 text-zinc-300 hover:bg-zinc-800 hover:text-white hover:border-zinc-600 focus:outline-none focus:ring-2 focus:ring-violet-500/40 transition-colors duration-150 {class}"
        )
    } else {
        let color = get_button_color_classes(color);
        format!(
            "h-[38px] w-full rounded-lg px-4 py-2 text-sm font-medium focus:outline-none focus:ring-2 focus:ring-violet-500/40 transition-colors duration-150 {color} {class}"
        )
    };
    view! { <button class=class>{children()}</button> }
}

#[component]
pub fn IconButton(
    #[prop(into, optional)] color: Option<Colors>,
    #[prop(into, optional)] ghost: bool,
    #[prop(into, optional)] class: String,
    #[prop(into)] icon: String,
) -> impl IntoView {
    let class = if ghost {
        let color = get_ghost_icon_button_color_classes(color);
        format!(
            "h-[34px] w-[34px] text-base rounded-lg flex items-center justify-center focus:outline-none transition-colors duration-150 {color} {class}"
        )
    } else {
        let color = get_button_color_classes(color);
        format!(
            "h-[38px] w-[38px] text-base rounded-lg flex items-center justify-center focus:outline-none focus:ring-2 focus:ring-violet-500/40 transition-colors duration-150 {color} {class}"
        )
    };
    view! {
        <button class=class>
            <i class=icon></i>
        </button>
    }
}
