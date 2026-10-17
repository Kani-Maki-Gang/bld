use bld_runner::{Dag, DagNode, job::v3::Needs, pipeline::v3::Pipeline};
use leptos::*;
use std::collections::HashMap;

const NODE_W: f64 = 180.0;
const NODE_H: f64 = 48.0;
const COL_GAP: f64 = 240.0;
const ROW_GAP: f64 = 80.0;
const PAD_X: f64 = 20.0;
const PAD_Y: f64 = 20.0;

#[derive(Clone, Debug, PartialEq)]
pub struct WorkflowNode {
    pub name: String,
    pub runs_on: String,
    pub steps: usize,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorkflowGraph {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<(usize, usize)>,
    pub stages: Vec<Vec<String>>,
    pub width: f64,
    pub height: f64,
}

impl WorkflowGraph {
    pub fn new(pipeline: &Pipeline) -> Self {
        let mut names: Vec<&String> = pipeline.jobs.keys().collect();
        names.sort();
        let index: HashMap<&str, usize> = names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();

        let nodes: Vec<DagNode> = names
            .iter()
            .map(|name| {
                let job = &pipeline.jobs[*name];
                let needs = match job.needs.as_ref() {
                    Some(Needs::Single(need)) => vec![need.clone()],
                    Some(Needs::Multiple(needs)) => needs.iter().cloned().collect(),
                    None => vec![],
                };
                DagNode {
                    name: (*name).clone(),
                    id: job.id.clone(),
                    condition: job.condition.clone(),
                    needs,
                }
            })
            .collect();

        let mut edges = vec![Vec::new(); nodes.len()];
        for (node_idx, node) in nodes.iter().enumerate() {
            for need in &node.needs {
                if let Some(&dep_idx) = index.get(need.as_str()) {
                    edges[dep_idx].push(node_idx);
                }
            }
        }

        let mut dag = Dag {
            nodes,
            edges,
            ..Default::default()
        };
        dag.reduce_edges();
        let layers = dag.layers();

        let mut position: HashMap<usize, f64> = HashMap::new();
        let mut stages: Vec<Vec<usize>> = Vec::with_capacity(layers.len());
        for layer in layers {
            let mut layer: Vec<usize> = layer.iter().map(|n| index[n.as_str()]).collect();
            let weight = |idx: usize| {
                let parents: Vec<f64> = dag
                    .edges
                    .iter()
                    .enumerate()
                    .filter(|(_, deps)| deps.contains(&idx))
                    .filter_map(|(p, _)| position.get(&p).copied())
                    .collect();
                if parents.is_empty() {
                    0.0
                } else {
                    parents.iter().sum::<f64>() / parents.len() as f64
                }
            };
            layer.sort_by(|a, b| {
                weight(*a)
                    .total_cmp(&weight(*b))
                    .then_with(|| names[*a].cmp(names[*b]))
            });
            for (row, idx) in layer.iter().enumerate() {
                position.insert(*idx, row as f64);
            }
            stages.push(layer);
        }

        let max_rows = stages.iter().map(|s| s.len()).max().unwrap_or(0);
        let mut layout: Vec<(usize, f64, f64)> = Vec::new();
        for (col, stage) in stages.iter().enumerate() {
            let offset = (max_rows - stage.len()) as f64 * ROW_GAP / 2.0;
            for (row, idx) in stage.iter().enumerate() {
                let x = PAD_X + col as f64 * COL_GAP;
                let y = PAD_Y + offset + row as f64 * ROW_GAP;
                layout.push((*idx, x, y));
            }
        }

        let order: HashMap<usize, usize> = layout
            .iter()
            .enumerate()
            .map(|(i, (idx, _, _))| (*idx, i))
            .collect();

        let graph_nodes = layout
            .iter()
            .map(|(idx, x, y)| {
                let job = &pipeline.jobs[names[*idx]];
                WorkflowNode {
                    name: names[*idx].clone(),
                    runs_on: job.runs_on.to_string(),
                    steps: job.steps.len(),
                    x: *x,
                    y: *y,
                }
            })
            .collect();

        let graph_edges = dag
            .edges
            .iter()
            .enumerate()
            .flat_map(|(from, deps)| deps.iter().map(move |to| (from, *to)))
            .filter_map(|(from, to)| Some((*order.get(&from)?, *order.get(&to)?)))
            .collect();

        let columns = stages.len();
        Self {
            nodes: graph_nodes,
            edges: graph_edges,
            stages: stages
                .into_iter()
                .map(|s| s.into_iter().map(|i| names[i].clone()).collect())
                .collect(),
            width: if columns == 0 {
                0.0
            } else {
                PAD_X * 2.0 + (columns - 1) as f64 * COL_GAP + NODE_W
            },
            height: if max_rows == 0 {
                0.0
            } else {
                PAD_Y * 2.0 + (max_rows - 1) as f64 * ROW_GAP + NODE_H
            },
        }
    }

    pub fn path(&self) -> String {
        self.edges
            .iter()
            .map(|(from, to)| {
                let a = &self.nodes[*from];
                let b = &self.nodes[*to];
                let (x1, y1) = (a.x + NODE_W, a.y + NODE_H / 2.0);
                let (x2, y2) = (b.x, b.y + NODE_H / 2.0);
                format!(
                    "M{x1} {y1}C{} {y1} {} {y2} {x2} {y2} ",
                    x1 + 34.0,
                    x2 - 34.0
                )
            })
            .collect()
    }
}

#[component]
pub fn WorkflowGraphView(#[prop(into)] graph: Signal<WorkflowGraph>) -> impl IntoView {
    view! {
        <div class="overflow-x-auto px-5 pt-[22px] pb-6">
            <div
                class="relative"
                style=move || {
                    graph.with(|g| format!("width:{}px;height:{}px", g.width, g.height))
                }
            >
                <svg
                    class="absolute left-0 top-0"
                    width=move || graph.with(|g| g.width)
                    height=move || graph.with(|g| g.height)
                >
                    <path
                        d=move || graph.with(|g| g.path())
                        fill="none"
                        stroke="#2b3040"
                        stroke-width="1.5"
                        stroke-linecap="round"
                    ></path>
                </svg>
                <For each=move || graph.with(|g| g.nodes.clone()) key=|n| n.name.clone() let:node>
                    <div
                        class="absolute w-[180px] h-12 border border-line-edge bg-hover rounded-[10px] px-[11px] flex flex-col justify-center gap-[3px]"
                        style=format!("left:{}px;top:{}px", node.x, node.y)
                    >
                        <div class="flex items-center gap-[7px] min-w-0">
                            <span class="size-[5px] rounded-full bg-ink-faint shrink-0"></span>
                            <span class="font-mono text-xs truncate" title=node.name.clone()>
                                {node.name.clone()}
                            </span>
                        </div>
                        <div class="text-[10.5px] text-ink-faint pl-3 truncate">
                            {format!(
                                "{} · {} {}",
                                node.runs_on,
                                node.steps,
                                if node.steps == 1 { "step" } else { "steps" },
                            )}
                        </div>
                    </div>
                </For>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pipeline(yaml: &str) -> Pipeline {
        serde_yaml_ng::from_str(yaml).unwrap()
    }

    #[test]
    fn graph_resolves_needs_into_stages() {
        let graph = WorkflowGraph::new(&pipeline(
            r#"
jobs:
  fetch:
    runs_on: machine
    steps: []
  resolve:
    runs_on: machine
    steps: []
  dispatch:
    runs_on: machine
    needs: [fetch, resolve]
    steps: []
  notify:
    runs_on: machine
    needs: [dispatch, fetch]
    steps: []
"#,
        ));
        assert_eq!(
            graph.stages,
            vec![
                vec!["fetch".to_string(), "resolve".to_string()],
                vec!["dispatch".to_string()],
                vec!["notify".to_string()],
            ]
        );
        assert_eq!(graph.edges.len(), 3);
        assert_eq!(graph.width, PAD_X * 2.0 + 2.0 * COL_GAP + NODE_W);
        assert_eq!(graph.height, PAD_Y * 2.0 + ROW_GAP + NODE_H);
    }

    #[test]
    fn graph_ignores_unknown_needs() {
        let graph = WorkflowGraph::new(&pipeline(
            r#"
jobs:
  build:
    runs_on: machine
    needs: missing
    steps: []
"#,
        ));
        assert_eq!(graph.stages, vec![vec!["build".to_string()]]);
        assert!(graph.edges.is_empty());
    }
}
