use leptos::*;

#[derive(Clone, Copy, Default, PartialEq)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Danger,
    DangerGhost,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum ButtonSize {
    Xs,
    Sm,
    #[default]
    Md,
    Lg,
}

pub fn button_classes(variant: ButtonVariant, size: ButtonSize) -> String {
    let size = match size {
        ButtonSize::Xs => "h-[26px] px-[9px] rounded-[7px] text-[11.5px]",
        ButtonSize::Sm => "h-[28px] px-[11px] rounded-lg text-xs",
        ButtonSize::Md => "h-8 px-[13px] rounded-[9px] text-[12.5px]",
        ButtonSize::Lg => "h-9 px-[15px] rounded-[9px] text-[13px]",
    };
    let variant = match variant {
        ButtonVariant::Primary => {
            "border-0 bg-accent text-white font-semibold hover:bg-accent-hover"
        }
        ButtonVariant::Secondary => {
            "border border-line-bright bg-raised text-ink font-medium hover:border-line-brighter"
        }
        ButtonVariant::Ghost => {
            "border border-line-strong bg-transparent text-ink-soft hover:text-ink hover:border-line-bright"
        }
        ButtonVariant::Danger => {
            "border border-bad-line bg-bad/[.08] text-bad font-medium hover:bg-bad/[.16]"
        }
        ButtonVariant::DangerGhost => {
            "border border-line-strong bg-transparent text-ink-muted hover:text-bad hover:border-bad-line"
        }
    };
    format!(
        "inline-flex items-center justify-center gap-1.5 whitespace-nowrap cursor-pointer transition-colors duration-150 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 disabled:cursor-not-allowed {size} {variant}"
    )
}

#[component]
pub fn Button(
    #[prop(into, optional)] variant: ButtonVariant,
    #[prop(into, optional)] size: ButtonSize,
    #[prop(into, optional)] class: String,
    #[prop(into, optional)] title: MaybeSignal<String>,
    #[prop(into, optional)] disabled: MaybeSignal<bool>,
    children: Children,
) -> impl IntoView {
    let class = format!("{} {class}", button_classes(variant, size));
    view! {
        <button class=class title=move || title.get() prop:disabled=move || disabled.get()>
            {children()}
        </button>
    }
}

#[component]
pub fn IconButton(
    #[prop(into, optional)] variant: Option<ButtonVariant>,
    #[prop(into, optional)] class: String,
    #[prop(into, optional)] title: String,
    #[prop(into)] icon: String,
) -> impl IntoView {
    let variant = match variant.unwrap_or(ButtonVariant::Ghost) {
        ButtonVariant::DangerGhost | ButtonVariant::Danger => {
            "text-ink-muted hover:text-bad hover:border-bad-line"
        }
        _ => "text-ink-muted hover:text-ink hover:border-line-bright",
    };
    let class = format!(
        "size-7 inline-flex items-center justify-center rounded-lg border border-line-strong bg-transparent text-[13px] cursor-pointer transition-colors duration-150 {variant} {class}"
    );
    view! {
        <button class=class title=title>
            <i class=icon></i>
        </button>
    }
}
