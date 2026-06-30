use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use {
    crate::{job::v3::Needs, pipeline::v3::Pipeline},
    anyhow::{Result, bail},
    std::collections::{HashMap, VecDeque},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DagRoot {
    pub name: Option<String>,
    pub cron: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    pub name: String,
    pub id: String,
    pub condition: Option<String>,
    pub needs: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dag {
    pub root: DagRoot,
    pub nodes: Vec<DagNode>,
    pub edges: Vec<Vec<usize>>,
}

impl Dag {
    #[cfg(feature = "all")]
    fn ensure_acyclic(&self) -> Result<()> {
        let order = self.topological_order();
        if order.len() != self.nodes.len() {
            let remaining: Vec<&str> = (0..self.nodes.len())
                .filter(|i| !order.contains(i))
                .map(|i| self.nodes[i].name.as_str())
                .collect();
            bail!(
                "pipeline jobs contain a cyclic dependency between: {}",
                remaining.join(", ")
            );
        }
        Ok(())
    }

    /// Computes a topological ordering of the node indices using Kahn's algorithm.
    #[cfg(feature = "all")]
    fn topological_order(&self) -> Vec<usize> {
        let mut in_degree = self.in_degrees();
        let mut queue: VecDeque<usize> = (0..self.nodes.len())
            .filter(|&i| in_degree[i] == 0)
            .collect();

        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(idx) = queue.pop_front() {
            order.push(idx);
            for &dependent in &self.edges[idx] {
                in_degree[dependent] -= 1;
                if in_degree[dependent] == 0 {
                    queue.push_back(dependent);
                }
            }
        }
        order
    }

    /// Groups the jobs into execution layers. All jobs within a layer can run in
    /// parallel and every layer must complete before the next one is started.
    pub fn layers(&self) -> Vec<Vec<String>> {
        let mut in_degree = self.in_degrees();
        let mut current: Vec<usize> = (0..self.nodes.len())
            .filter(|&i| in_degree[i] == 0)
            .collect();

        let mut layers = Vec::new();
        while !current.is_empty() {
            let mut next = Vec::new();
            let mut layer = Vec::with_capacity(current.len());
            for &idx in &current {
                layer.push(self.nodes[idx].name.clone());
                for &dependent in &self.edges[idx] {
                    in_degree[dependent] -= 1;
                    if in_degree[dependent] == 0 {
                        next.push(dependent);
                    }
                }
            }
            layers.push(layer);
            current = next;
        }
        layers
    }

    /// Performs a transitive reduction of the graph, removing every edge that is
    /// already implied by a longer path. The reachability of all nodes is kept
    /// intact while producing a more optimized graph for execution and rendering.
    pub fn reduce_edges(&mut self) {
        let len = self.nodes.len();
        let reachable: Vec<HashSet<usize>> = (0..len).map(|idx| self.reachable_from(idx)).collect();

        let mut reduced = vec![Vec::new(); len];
        for (node, dependents) in self.edges.iter().enumerate() {
            for &dependent in dependents {
                // Drop the edge when the dependent is already reachable through
                // another direct successor of the current node.
                let redundant = dependents
                    .iter()
                    .any(|&other| other != dependent && reachable[other].contains(&dependent));
                if !redundant {
                    reduced[node].push(dependent);
                }
            }
        }
        self.edges = reduced;
    }

    fn in_degrees(&self) -> Vec<usize> {
        let mut in_degree = vec![0usize; self.nodes.len()];
        for dependents in &self.edges {
            for &dependent in dependents {
                in_degree[dependent] += 1;
            }
        }
        in_degree
    }

    /// Collects every node reachable from `start`, excluding `start` itself.
    fn reachable_from(&self, start: usize) -> HashSet<usize> {
        let mut visited = HashSet::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            for &next in &self.edges[node] {
                if visited.insert(next) {
                    stack.push(next);
                }
            }
        }
        visited
    }
}

#[cfg(feature = "all")]
impl TryFrom<&Pipeline> for Dag {
    type Error = anyhow::Error;

    fn try_from(pipeline: &Pipeline) -> Result<Self, Self::Error> {
        let mut nodes = Vec::with_capacity(pipeline.jobs.len());
        let mut index = HashMap::with_capacity(pipeline.jobs.len());

        for (name, job) in &pipeline.jobs {
            index.insert(name.clone(), nodes.len());
            let needs = match job.needs.as_ref() {
                Some(Needs::Single(need)) => vec![need.clone()],
                Some(Needs::Multiple(needs)) => needs.iter().cloned().collect(),
                None => vec![],
            };
            nodes.push(DagNode {
                name: name.clone(),
                id: job.id.clone(),
                condition: job.condition.clone(),
                needs,
            });
        }

        let mut edges = vec![Vec::new(); nodes.len()];
        for (node_idx, node) in nodes.iter().enumerate() {
            for need in &node.needs {
                let Some(&dep_idx) = index.get(need) else {
                    bail!("job '{}' depends on undefined job '{}'", node.name, need);
                };
                // The dependency must run before the current node.
                edges[dep_idx].push(node_idx);
            }
        }

        let root = DagRoot {
            name: pipeline.name.clone(),
            cron: pipeline.cron.clone(),
        };

        let dag = Self { root, nodes, edges };
        dag.ensure_acyclic()?;
        Ok(dag)
    }
}

#[cfg(all(test, feature = "all"))]
mod tests {
    use std::collections::HashSet;

    use crate::{
        job::v3::{Job, Needs},
        pipeline::v3::Pipeline,
    };

    use super::Dag;

    fn job(needs: &[&str]) -> Job {
        let needs = match needs {
            [] => None,
            [single] => Some(Needs::Single(single.to_string())),
            many => Some(Needs::Multiple(
                many.iter().map(|n| n.to_string()).collect::<HashSet<_>>(),
            )),
        };
        Job {
            needs,
            ..Job::default()
        }
    }

    fn pipeline(jobs: &[(&str, Job)]) -> Pipeline {
        Pipeline {
            jobs: jobs
                .iter()
                .map(|(name, job)| (name.to_string(), job.clone()))
                .collect(),
            ..Pipeline::default()
        }
    }

    fn sorted_layers(dag: &Dag) -> Vec<Vec<String>> {
        dag.layers()
            .into_iter()
            .map(|mut layer| {
                layer.sort();
                layer
            })
            .collect()
    }

    fn sorted_edges(dag: &Dag) -> Vec<(String, String)> {
        let mut edges = Vec::new();
        for (from, dependents) in dag.edges.iter().enumerate() {
            for &to in dependents {
                edges.push((dag.nodes[from].name.clone(), dag.nodes[to].name.clone()));
            }
        }
        edges.sort();
        edges
    }

    fn node_idx(dag: &Dag, name: &str) -> usize {
        dag.nodes
            .iter()
            .position(|n| n.name == name)
            .unwrap_or_else(|| panic!("node '{name}' not found"))
    }

    #[test]
    fn empty_pipeline_builds_empty_dag() {
        let dag = Dag::try_from(&pipeline(&[])).expect("empty pipeline is valid");
        assert!(dag.nodes.is_empty());
        assert!(dag.edges.is_empty());
        assert!(dag.layers().is_empty());
    }

    #[test]
    fn single_job_has_no_edges_and_copies_root() {
        let mut p = pipeline(&[("a", job(&[]))]);
        p.name = Some("my-pipeline".to_string());
        p.cron = Some("0 0 * * * *".to_string());

        let dag = Dag::try_from(&p).expect("single job is valid");

        assert_eq!(dag.nodes.len(), 1);
        assert!(dag.edges.iter().all(|e| e.is_empty()));
        assert_eq!(sorted_layers(&dag), vec![vec!["a".to_string()]]);
        assert_eq!(dag.root.name.as_deref(), Some("my-pipeline"));
        assert_eq!(dag.root.cron.as_deref(), Some("0 0 * * * *"));
    }

    #[test]
    fn single_need_produces_one_edge() {
        let dag = Dag::try_from(&pipeline(&[("a", job(&[])), ("b", job(&["a"]))]))
            .expect("valid dependency");

        let a = node_idx(&dag, "a");
        assert_eq!(dag.nodes[a].needs, Vec::<String>::new());
        assert_eq!(dag.nodes[node_idx(&dag, "b")].needs, vec!["a".to_string()]);
        assert_eq!(sorted_edges(&dag), vec![("a".to_string(), "b".to_string())]);
    }

    #[test]
    fn multiple_needs_produce_all_edges() {
        let dag = Dag::try_from(&pipeline(&[
            ("a", job(&[])),
            ("b", job(&[])),
            ("c", job(&[])),
            ("d", job(&["b", "c"])),
        ]))
        .expect("valid dependencies");

        assert_eq!(
            sorted_edges(&dag),
            vec![
                ("b".to_string(), "d".to_string()),
                ("c".to_string(), "d".to_string()),
            ]
        );
    }

    #[test]
    fn linear_chain_layers_in_order() {
        let dag = Dag::try_from(&pipeline(&[
            ("a", job(&[])),
            ("b", job(&["a"])),
            ("c", job(&["b"])),
        ]))
        .expect("valid chain");

        assert_eq!(
            sorted_edges(&dag),
            vec![
                ("a".to_string(), "b".to_string()),
                ("b".to_string(), "c".to_string()),
            ]
        );
        assert_eq!(
            sorted_layers(&dag),
            vec![
                vec!["a".to_string()],
                vec!["b".to_string()],
                vec!["c".to_string()],
            ]
        );
    }

    #[test]
    fn diamond_layers_correctly() {
        let dag = Dag::try_from(&pipeline(&[
            ("a", job(&[])),
            ("b", job(&["a"])),
            ("c", job(&["a"])),
            ("d", job(&["b", "c"])),
        ]))
        .expect("valid diamond");

        assert_eq!(
            sorted_layers(&dag),
            vec![
                vec!["a".to_string()],
                vec!["b".to_string(), "c".to_string()],
                vec!["d".to_string()],
            ]
        );
    }

    #[test]
    fn undefined_dependency_is_rejected() {
        let err = Dag::try_from(&pipeline(&[("a", job(&["missing"]))]))
            .expect_err("undefined dependency must fail");
        assert!(
            err.to_string().contains("depends on undefined job"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn self_cycle_is_rejected() {
        let err =
            Dag::try_from(&pipeline(&[("a", job(&["a"]))])).expect_err("self cycle must fail");
        assert!(
            err.to_string().contains("cyclic dependency"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn two_node_cycle_is_rejected() {
        let err = Dag::try_from(&pipeline(&[("a", job(&["b"])), ("b", job(&["a"]))]))
            .expect_err("two node cycle must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("cyclic dependency"),
            "unexpected message: {msg}"
        );
        assert!(
            msg.contains('a') && msg.contains('b'),
            "should name jobs: {msg}"
        );
    }

    #[test]
    fn longer_cycle_is_rejected() {
        let err = Dag::try_from(&pipeline(&[
            ("a", job(&["c"])),
            ("b", job(&["a"])),
            ("c", job(&["b"])),
        ]))
        .expect_err("longer cycle must fail");
        assert!(
            err.to_string().contains("cyclic dependency"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn independent_roots_share_first_layer() {
        let dag = Dag::try_from(&pipeline(&[("a", job(&[])), ("b", job(&[]))])).expect("valid");
        assert_eq!(
            sorted_layers(&dag),
            vec![vec!["a".to_string(), "b".to_string()]]
        );
    }

    #[test]
    fn disconnected_components_layer_independently() {
        let dag = Dag::try_from(&pipeline(&[
            ("a", job(&[])),
            ("b", job(&["a"])),
            ("c", job(&[])),
            ("d", job(&["c"])),
        ]))
        .expect("valid");
        assert_eq!(
            sorted_layers(&dag),
            vec![
                vec!["a".to_string(), "c".to_string()],
                vec!["b".to_string(), "d".to_string()],
            ]
        );
    }

    #[test]
    fn reduce_drops_redundant_transitive_edge() {
        let mut dag = Dag::try_from(&pipeline(&[
            ("a", job(&[])),
            ("b", job(&["a"])),
            ("c", job(&["a", "b"])),
        ]))
        .expect("valid");

        assert_eq!(
            sorted_edges(&dag),
            vec![
                ("a".to_string(), "b".to_string()),
                ("a".to_string(), "c".to_string()),
                ("b".to_string(), "c".to_string()),
            ]
        );

        dag.reduce_edges();

        assert_eq!(
            sorted_edges(&dag),
            vec![
                ("a".to_string(), "b".to_string()),
                ("b".to_string(), "c".to_string()),
            ]
        );
    }

    #[test]
    fn reduce_preserves_diamond() {
        let mut dag = Dag::try_from(&pipeline(&[
            ("a", job(&[])),
            ("b", job(&["a"])),
            ("c", job(&["a"])),
            ("d", job(&["b", "c"])),
        ]))
        .expect("valid");

        let before = sorted_edges(&dag);
        dag.reduce_edges();
        assert_eq!(sorted_edges(&dag), before);
    }

    #[test]
    fn reduce_preserves_layers() {
        let mut dag = Dag::try_from(&pipeline(&[
            ("a", job(&[])),
            ("b", job(&["a"])),
            ("c", job(&["a", "b"])),
        ]))
        .expect("valid");

        let before = sorted_layers(&dag);
        dag.reduce_edges();
        assert_eq!(sorted_layers(&dag), before);
    }

    #[test]
    fn reduce_is_idempotent() {
        let mut once = Dag::try_from(&pipeline(&[
            ("a", job(&[])),
            ("b", job(&["a"])),
            ("c", job(&["a", "b"])),
        ]))
        .expect("valid");
        once.reduce_edges();
        let after_once = sorted_edges(&once);

        once.reduce_edges();
        assert_eq!(sorted_edges(&once), after_once);
    }
}
