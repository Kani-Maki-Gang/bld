use crate::components::{
    card::{Card, CardHeader},
    dag::{WorkflowGraph, WorkflowGraphView},
};
use leptos::*;

#[component]
pub fn PipelineWorkflow(#[prop()] graph: WorkflowGraph) -> impl IntoView {
    let jobs = graph.nodes.len();
    let stages = graph.stages.len();
    let max_parallel = graph.stages.iter().map(|s| s.len()).max().unwrap_or(0);
    let subtitle = format!(
        "{jobs} {}, resolved from needs: into {stages} {}. Jobs in the same stage run in parallel.",
        if jobs == 1 { "job" } else { "jobs" },
        if stages == 1 { "stage" } else { "stages" },
    );
    let (graph, _) = create_signal(graph);
    view! {
        <Card class="mt-[18px] overflow-hidden">
            <CardHeader title="Workflow" subtitle=subtitle>
                <span class="text-[11.5px] text-ink-muted">
                    {format!("up to {max_parallel} jobs in parallel")}
                </span>
            </CardHeader>
            <WorkflowGraphView graph=graph />
            <div class="px-5 py-[13px] border-t border-line-soft text-[11.5px] text-ink-muted">
                "Stages run left to right. Each node shows where the job runs and its step count."
            </div>
        </Card>
    }
}
