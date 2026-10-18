use crate::components::card::CardHeader;
use bld_runner::{
    VersionedFile, files::v3::RunnerFile, inputs::v3::input_default_text, traits::IntoVariables,
};
use leptos::*;
use std::collections::HashMap;

#[derive(Clone)]
pub struct InputField {
    pub name: String,
    pub hint: String,
    pub default: String,
    pub required: bool,
    pub value: RwSignal<String>,
}

impl InputField {
    fn new(
        name: String,
        hint: String,
        default: String,
        required: bool,
        value: Option<String>,
    ) -> Self {
        let value = create_rw_signal(value.unwrap_or_else(|| default.clone()));
        Self {
            name,
            hint,
            default,
            required,
            value,
        }
    }

    pub fn is_missing(&self) -> bool {
        self.required && self.value.with(|v| v.trim().is_empty())
    }
}

fn sorted(mut fields: Vec<InputField>) -> Vec<InputField> {
    fields.sort_by(|a, b| {
        b.required
            .cmp(&a.required)
            .then_with(|| a.name.cmp(&b.name))
    });
    fields
}

fn simple_fields(
    items: HashMap<String, String>,
    overrides: &mut HashMap<String, String>,
) -> Vec<InputField> {
    sorted(
        items
            .into_iter()
            .map(|(name, default)| {
                let value = overrides.remove(&name);
                InputField::new(name, String::new(), default, false, value)
            })
            .collect(),
    )
}

pub fn file_fields(
    file: VersionedFile,
    inputs: Option<HashMap<String, String>>,
    env: Option<HashMap<String, String>>,
) -> (Vec<InputField>, Vec<InputField>) {
    let mut inputs = inputs.unwrap_or_default();
    let mut env = env.unwrap_or_default();

    if let VersionedFile::Version3(RunnerFile::PipelineFileType(pipeline)) = &file {
        let fields = pipeline
            .inputs
            .iter()
            .map(|(name, input)| {
                let hint = input
                    .description
                    .as_deref()
                    .map(|d| d.trim().to_string())
                    .unwrap_or_default();
                let default = input_default_text(input);
                let value = inputs.remove(name);
                InputField::new(name.clone(), hint, default, input.is_required(), value)
            })
            .collect();
        let env_fields = simple_fields(pipeline.env.clone(), &mut env);
        return (sorted(fields), env_fields);
    }

    let (vars, environment) = file.into_variables();
    (
        simple_fields(vars.unwrap_or_default(), &mut inputs),
        simple_fields(environment.unwrap_or_default(), &mut env),
    )
}

pub fn field_values(fields: &[InputField]) -> HashMap<String, String> {
    fields
        .iter()
        .map(|f| (f.name.clone(), f.value.get_untracked()))
        .collect()
}

#[component]
fn FieldRow(#[prop()] field: InputField, #[prop(into)] attempted: Signal<bool>) -> impl IntoView {
    let value = field.value;
    let required = field.required;
    let default = field.default.clone();
    let empty = move || value.with(|v| v.trim().is_empty());
    let missing = move || required && empty();
    let error = move || missing() && attempted.get();
    let changed = {
        let default = default.clone();
        move || !empty() && value.with(|v| *v != default)
    };

    let badge = move || {
        if error() {
            ("required", "bg-bad/[.14] text-bad")
        } else if missing() {
            ("required", "bg-warn/[.12] text-warn")
        } else {
            ("required ✓", "bg-ok/10 text-ok")
        }
    };
    let input_class = {
        let changed = changed.clone();
        move || {
            let state = if error() {
                "border-bad-strong bg-[#1c1416] shadow-[0_0_0_3px_rgba(240,96,79,.10)] text-ink"
            } else if missing() {
                "border-warn-line bg-raised text-ink"
            } else if changed() {
                "border-accent-line bg-raised text-accent-ink"
            } else {
                "border-line-strong bg-raised text-ink"
            };
            format!(
                "w-full h-[34px] rounded-[9px] border px-[11px] font-mono text-[12.5px] outline-none placeholder:text-ink-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(108,92,245,.14)] {state}"
            )
        }
    };
    let message = {
        let changed = changed.clone();
        move || {
            if error() {
                Some((
                    "A value is required before the run can start.".to_string(),
                    "text-bad-soft",
                ))
            } else if missing() {
                Some((
                    "No default. Set this to start the run.".to_string(),
                    "text-warn-soft",
                ))
            } else if changed() {
                let default = if default.is_empty() {
                    "empty".to_string()
                } else {
                    default.clone()
                };
                Some((format!("Changed from default: {default}"), "text-ink-muted"))
            } else {
                None
            }
        }
    };
    let placeholder = if required {
        "Enter a value"
    } else {
        "Optional"
    };

    view! {
        <div class=move || {
            format!(
                "px-5 py-3 border-b border-line-faint grid grid-cols-[minmax(120px,190px)_minmax(0,1fr)] gap-3.5 items-start {}",
                if error() { "bg-bad/[.03]" } else { "" },
            )
        }>
            <div class="min-w-0 pt-1">
                <div class="flex items-center gap-[7px] flex-wrap">
                    <span class="font-mono text-[12.5px] break-all">{field.name.clone()}</span>
                    <Show when=move || required fallback=|| view! {}>
                        <span class=move || {
                            format!(
                                "inline-flex items-center h-[17px] px-1.5 rounded-[5px] text-[10px] font-semibold tracking-[.04em] uppercase {}",
                                badge().1,
                            )
                        }>{move || badge().0}</span>
                    </Show>
                </div>
                <div
                    class="text-[11px] text-ink-faint mt-0.5 line-clamp-2"
                    title=field.hint.clone()
                >
                    {field.hint.clone()}
                </div>
            </div>
            <div class="min-w-0">
                <input
                    class=input_class
                    placeholder=placeholder
                    prop:value=move || value.get()
                    on:input=move |ev| value.set(event_target_value(&ev))
                />
                {move || {
                    message()
                        .map(|(text, class)| {
                            view! {
                                <div class=format!("mt-1.5 text-[11.5px] {class}")>{text}</div>
                            }
                        })
                }}
            </div>
        </div>
    }
}

#[component]
pub fn RunPipelineVariables(
    #[prop(into)] title: String,
    #[prop(into)] subtitle: String,
    #[prop()] fields: Vec<InputField>,
    #[prop(into, optional)] attempted: Signal<bool>,
) -> impl IntoView {
    let required: Vec<InputField> = fields.iter().filter(|f| f.required).cloned().collect();
    let total = required.len();
    let missing = {
        let required = required.clone();
        move || required.iter().filter(|f| f.is_missing()).count()
    };
    let summary = {
        let missing = missing.clone();
        move || {
            let count = missing();
            if count == 0 {
                (
                    format!("{total} of {total} required set"),
                    "bg-ok/10 text-ok",
                    "bg-ok",
                )
            } else if attempted.get() {
                (
                    format!(
                        "{count} required {} missing",
                        if count == 1 { "input" } else { "inputs" }
                    ),
                    "bg-bad/[.12] text-bad",
                    "bg-bad",
                )
            } else {
                (
                    format!(
                        "{count} required {} missing",
                        if count == 1 { "input" } else { "inputs" }
                    ),
                    "bg-warn/[.12] text-warn",
                    "bg-warn",
                )
            }
        }
    };
    let summary = Signal::derive(summary);
    let banner = {
        let missing = missing.clone();
        move || {
            let count = missing();
            (attempted.get() && count > 0).then(|| {
                format!(
                    "Fill in {count} required {} to start this run.",
                    if count == 1 { "input" } else { "inputs" }
                )
            })
        }
    };

    view! {
        <CardHeader title=title subtitle=subtitle>
            <Show when=move || { total > 0 } fallback=|| view! {}>
                <span class=move || {
                    format!(
                        "inline-flex items-center gap-[7px] h-6 px-2.5 rounded-[7px] text-xs font-medium {}",
                        summary.get().1,
                    )
                }>
                    <span class=move || format!("size-1.5 rounded-full {}", summary.get().2)></span>
                    {move || summary.get().0}
                </span>
            </Show>
        </CardHeader>
        {move || {
            banner()
                .map(|text| {
                    view! {
                        <div class="px-5 py-[11px] border-b border-[#3a2a30] bg-bad/[.07] flex items-center gap-2.5 text-[12.5px] text-bad-soft">
                            <span class="size-1.5 rounded-full bg-bad shrink-0"></span>
                            <span>{text}</span>
                        </div>
                    }
                })
        }}
        {fields
            .into_iter()
            .map(|field| view! { <FieldRow field=field attempted=attempted /> })
            .collect_view()}
    }
}
