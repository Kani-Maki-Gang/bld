use leptos::*;

pub enum ButtonMode {
    Default,
    Success,
    Danger
}

impl Default for ButtonMode {
    fn default() -> Self {
        ButtonMode::Default
    }
}

fn get_mode_classes(mode: ButtonMode) -> &'static str {
    match mode {
        ButtonMode::Default => "bg-indigo-600 hover:bg-indigo-700 focus:bg-indigo-700",
        ButtonMode::Success => "bg-green-600 hover:bg-green-700 focus:bg-green-700",
        ButtonMode::Danger => "bg-red-600 hover:bg-red-700 focus:bg-red-700",
    }
}

#[component]
pub fn Button(
    #[prop(optional)] mode: ButtonMode,
    #[prop(into, optional)] class: String,
    children: Children
) -> impl IntoView {
    let mode = get_mode_classes(mode);
    let class=format!("h-[40px] w-full flex-none rounded-lg p-2 focus:outline-none {mode} {class}");
    view! {
        <button class=class>
            {children()}
        </button>
    }
}

#[component]
pub fn IconButton(
    #[prop(optional)] mode: ButtonMode,
    #[prop(into, optional)] class: String,
    #[prop(into)] icon: String
) -> impl IntoView {
    let mode = get_mode_classes(mode);
    let class=format!("h-[40px] w-[40px] flex-none text-xl rounded-lg p-2 focus:outline-none {mode} {class}");
    view! {
        <button class=class>
            <i class=icon />
        </button>
    }
}
