use leptos::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Initial,
    Queued,
    Running,
    Finished,
    Faulted,
    NoRun,
}

impl RunState {
    pub fn parse(state: &str) -> Self {
        match state {
            "initial" => Self::Initial,
            "queued" => Self::Queued,
            "running" => Self::Running,
            "finished" => Self::Finished,
            "faulted" => Self::Faulted,
            _ => Self::NoRun,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Initial => "Initial",
            Self::Queued => "Queued",
            Self::Running => "Running",
            Self::Finished => "Success",
            Self::Faulted => "Faulted",
            Self::NoRun => "No runs",
        }
    }

    pub fn text_class(&self) -> &'static str {
        match self {
            Self::Initial | Self::NoRun => "text-ink-muted",
            Self::Queued => "text-warn",
            Self::Running => "text-run",
            Self::Finished => "text-ok",
            Self::Faulted => "text-bad",
        }
    }

    pub fn dot_class(&self) -> &'static str {
        match self {
            Self::Initial | Self::NoRun => "bg-ink-muted",
            Self::Queued => "bg-warn",
            Self::Running => "bg-run animate-bld-pulse",
            Self::Finished => "bg-ok",
            Self::Faulted => "bg-bad",
        }
    }

    pub fn tick_class(&self) -> &'static str {
        match self {
            Self::Initial | Self::Queued => "bg-warn",
            Self::Running => "bg-run",
            Self::Finished => "bg-ok",
            Self::Faulted => "bg-bad",
            Self::NoRun => "bg-line",
        }
    }

    pub fn badge_class(&self) -> &'static str {
        match self {
            Self::Initial | Self::NoRun => "bg-ink-muted/[.12] text-ink-muted",
            Self::Queued => "bg-warn/[.12] text-warn",
            Self::Running => "bg-run/[.12] text-run",
            Self::Finished => "bg-ok/[.12] text-ok",
            Self::Faulted => "bg-bad/[.12] text-bad",
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(self, Self::Finished | Self::Faulted)
    }
}

#[component]
pub fn Dot(#[prop(into)] class: MaybeSignal<String>) -> impl IntoView {
    view! { <span class=move || format!("inline-block shrink-0 rounded-full {}", class.get())></span> }
}

#[component]
pub fn StateBadge(#[prop(into)] state: MaybeSignal<RunState>) -> impl IntoView {
    view! {
        <span class=move || {
            format!(
                "shrink-0 inline-flex items-center h-[22px] px-[9px] rounded-[7px] text-[11.5px] font-medium {}",
                state.get().badge_class(),
            )
        }>{move || state.get().label()}</span>
    }
}

#[component]
pub fn Chip(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
    let class = format!(
        "inline-flex items-center gap-1.5 h-[21px] px-2 rounded-md bg-chip border border-line-strong text-[11.5px] text-ink-soft whitespace-nowrap {class}"
    );
    view! { <span class=class>{children()}</span> }
}

#[component]
pub fn TickStrip(
    #[prop(into)] states: MaybeSignal<Vec<RunState>>,
    #[prop(optional)] slots: Option<usize>,
    #[prop(into, optional)] tick_class: String,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let slots = slots.unwrap_or(30);
    let ticks = move || {
        let states = states.get();
        let skip = states.len().saturating_sub(slots);
        let mut ticks = vec![RunState::NoRun; slots.saturating_sub(states.len())];
        ticks.extend(states.into_iter().skip(skip));
        ticks.into_iter().enumerate().collect::<Vec<_>>()
    };
    let class = format!("flex gap-[2px] items-end {class}");
    view! {
        <div class=class>
            <For each=ticks key=|(i, s)| (*i, s.label()) let:tick>
                <div class=format!(
                    "flex-1 rounded-[1.5px] {} {}",
                    tick_class,
                    tick.1.tick_class(),
                )></div>
            </For>
        </div>
    }
}
