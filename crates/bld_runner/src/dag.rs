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
