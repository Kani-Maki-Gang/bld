use std::collections::HashMap;

use anyhow::{Result, anyhow, bail};
use uuid::Uuid;

use crate::expr::v3::traits::{ExprValue, OutputScope, WritableRuntimeExprContext};

#[cfg(test)]
use mockall::{automock, mock};

#[cfg_attr(test, automock)]
pub trait NodeState {
    fn update_state(&mut self, state: State);
    #[allow(unused)]
    fn get_state(&self) -> &State;
}

pub trait RootState: WritableRuntimeExprContext {
    fn update_state(&mut self, state: State);
    fn get_state(&self) -> &State;
    fn add_node(&mut self, node_id: &str);
    fn update_node_state(&mut self, node_id: &str, state: State);
    fn get_node_state<'a>(&'a self, node_id: &str) -> Option<&'a State>;
    fn set_matrix(&mut self, matrix: HashMap<String, String>);
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum State {
    #[default]
    Default,
    Running,
    Completed,
    Skipped,
    Failed {
        error: String,
    },
}

#[derive(Debug, Default, PartialEq)]
pub struct StepState {
    id: String,
    state: State,
    // TECH DEBT: Change 'static lifetime to a specific lifetime.
    // Changing this will require a lot of type annotation changes.
    outputs: HashMap<String, ExprValue<'static>>,
}

impl StepState {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            ..Default::default()
        }
    }
}

impl NodeState for StepState {
    fn update_state(&mut self, state: State) {
        self.state = state;
    }

    fn get_state(&self) -> &State {
        &self.state
    }
}

impl WritableRuntimeExprContext for StepState {
    fn get_exec_id(&self) -> Option<&str> {
        Some(self.id.as_str())
    }

    fn get_output<'a>(&'a self, scope: OutputScope, id: &str, name: &str) -> Result<ExprValue<'a>> {
        if scope != OutputScope::Step {
            bail!("job outputs are not accessible from step state");
        }
        if self.id != id {
            bail!("id {id} has no outputs");
        }
        self.outputs
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("output '{name}' not found"))
    }

    fn set_output(&mut self, id: &str, name: String, value: String) -> Result<()> {
        if self.id != id {
            bail!("target id {id} is inaccessible");
        }
        let _ = self.outputs.insert(name, value.try_into()?);
        Ok(())
    }

    fn set_outputs(&mut self, id: &str, outputs: HashMap<String, String>) -> Result<()> {
        if self.id != id {
            bail!("target id {id} is inaccessible");
        }
        let mut map = HashMap::new();
        for (k, v) in outputs {
            map.insert(k, v.try_into()?);
        }
        self.outputs = map;
        Ok(())
    }

    fn get_matrix_value<'a>(&'a self, _name: &str) -> Result<&'a str> {
        bail!("matrix values are not accessible from step state")
    }
}

#[derive(Debug, PartialEq)]
pub struct JobState {
    name: String,
    state: State,
    steps: HashMap<String, StepState>,
    matrix: HashMap<String, String>,
    // TECH DEBT: Change 'static lifetime to a specific lifetime.
    // Changing this will require a lot of type annotation changes.
    job_outputs: HashMap<String, HashMap<String, ExprValue<'static>>>,
}

impl JobState {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn set_job_outputs(
        &mut self,
        job_outputs: HashMap<String, HashMap<String, String>>,
    ) -> Result<()> {
        let mut map = HashMap::new();
        for (job, outputs) in job_outputs {
            let mut inner = HashMap::new();
            for (name, value) in outputs {
                inner.insert(name, value.try_into()?);
            }
            map.insert(job, inner);
        }
        self.job_outputs = map;
        Ok(())
    }
}

impl Default for JobState {
    fn default() -> Self {
        Self {
            name: Uuid::new_v4().to_string(),
            state: State::default(),
            steps: HashMap::new(),
            matrix: HashMap::new(),
            job_outputs: HashMap::new(),
        }
    }
}

impl RootState for JobState {
    fn update_state(&mut self, state: State) {
        self.state = state;
    }

    fn get_state(&self) -> &State {
        &self.state
    }

    fn add_node(&mut self, node_id: &str) {
        self.steps
            .insert(node_id.to_string(), StepState::new(node_id));
    }

    fn update_node_state(&mut self, node_id: &str, state: State) {
        let Some(step_state) = self.steps.get_mut(node_id) else {
            return;
        };
        step_state.update_state(state);
    }

    fn get_node_state<'a>(&'a self, node_id: &str) -> Option<&'a State> {
        self.steps.get(node_id).map(|x| &x.state)
    }

    fn set_matrix(&mut self, matrix: HashMap<String, String>) {
        self.matrix = matrix;
    }
}

impl WritableRuntimeExprContext for JobState {
    fn get_exec_id(&self) -> Option<&str> {
        Some(self.name.as_str())
    }

    fn get_output<'a>(&'a self, scope: OutputScope, id: &str, name: &str) -> Result<ExprValue<'a>> {
        match scope {
            OutputScope::Job => {
                let Some(outputs) = self.job_outputs.get(id) else {
                    bail!(
                        "job '{id}' not found, only jobs listed in 'needs' have their outputs available"
                    );
                };
                outputs
                    .get(name)
                    .cloned()
                    .ok_or_else(|| anyhow!("output '{name}' not found for job '{id}'"))
            }
            OutputScope::Step => {
                let Some(step_state) = self.steps.get(id) else {
                    bail!("outputs for id {id} weren't found");
                };
                step_state.get_output(OutputScope::Step, id, name)
            }
        }
    }

    fn set_output(&mut self, id: &str, name: String, value: String) -> Result<()> {
        let Some(step_state) = self.steps.get_mut(id) else {
            bail!("outputs for id {id} weren't found");
        };
        step_state.set_output(id, name, value)
    }

    fn set_outputs(&mut self, id: &str, outputs: HashMap<String, String>) -> Result<()> {
        let Some(step_state) = self.steps.get_mut(id) else {
            bail!("outputs for id {id} weren't found");
        };
        step_state.set_outputs(id, outputs)
    }

    fn get_matrix_value<'a>(&'a self, name: &str) -> Result<&'a str> {
        self.matrix
            .get(name)
            .map(|x| x.as_str())
            .ok_or_else(|| anyhow!("matrix value '{name}' not found"))
    }
}

pub struct ActionState {
    id: String,
    state: State,
    steps: HashMap<String, StepState>,
    matrix: HashMap<String, String>,
}

impl Default for ActionState {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            state: State::Default,
            steps: HashMap::new(),
            matrix: HashMap::new(),
        }
    }
}

impl RootState for ActionState {
    fn update_state(&mut self, state: State) {
        self.state = state;
    }

    fn get_state(&self) -> &State {
        &self.state
    }

    fn add_node(&mut self, node_id: &str) {
        self.steps
            .insert(node_id.to_string(), StepState::new(node_id));
    }

    fn update_node_state(&mut self, node_id: &str, state: State) {
        let Some(step_state) = self.steps.get_mut(node_id) else {
            return;
        };
        step_state.update_state(state);
    }

    fn get_node_state<'a>(&'a self, node_id: &str) -> Option<&'a State> {
        self.steps.get(node_id).map(|x| &x.state)
    }

    fn set_matrix(&mut self, matrix: HashMap<String, String>) {
        self.matrix = matrix;
    }
}

impl WritableRuntimeExprContext for ActionState {
    fn get_exec_id(&self) -> Option<&str> {
        Some(self.id.as_str())
    }

    fn get_output<'a>(&'a self, scope: OutputScope, id: &str, name: &str) -> Result<ExprValue<'a>> {
        if scope != OutputScope::Step {
            bail!("jobs are not accessible from an action");
        }
        let Some(step_state) = self.steps.get(id) else {
            bail!("outputs for id {id} weren't found");
        };
        step_state.get_output(OutputScope::Step, id, name)
    }

    fn set_output(&mut self, id: &str, name: String, value: String) -> Result<()> {
        let Some(step_state) = self.steps.get_mut(id) else {
            bail!("outputs for id {id} weren't found");
        };
        step_state.set_output(id, name, value)
    }

    fn set_outputs(&mut self, id: &str, outputs: HashMap<String, String>) -> Result<()> {
        let Some(step_state) = self.steps.get_mut(id) else {
            bail!("outputs for id {id} weren't found");
        };
        step_state.set_outputs(id, outputs)
    }

    fn get_matrix_value<'a>(&'a self, name: &str) -> Result<&'a str> {
        self.matrix
            .get(name)
            .map(|x| x.as_str())
            .ok_or_else(|| anyhow!("matrix value '{name}' not found"))
    }
}

#[cfg(test)]
mod mocks {
    use super::*;

    mock! {
        pub RootState {}

        impl RootState for RootState {
            fn update_state(&mut self, state: State);
            fn get_state(&self) -> &State;
            fn add_node(&mut self, node_id: &str);
            fn update_node_state(&mut self, node_id: &str, state: State);
            fn get_node_state<'a>(&'a self, node_id: &str) -> Option<&'a State>;
            fn set_matrix(&mut self, matrix: HashMap<String, String>);
        }

        impl WritableRuntimeExprContext for RootState {
            fn get_exec_id<'a> (&'a self) -> Option<&'a str>;
            fn get_output<'a>(&'a self, scope: OutputScope, id: &str, name: &str) -> Result<ExprValue<'a>>;
            fn set_output(&mut self, id: &str, name: String, value: String) -> Result<()>;
            fn set_outputs(&mut self, id: &str, outputs: HashMap<String, String>) -> Result<()>;
            fn get_matrix_value<'a>(&'a self, name: &str) -> Result<&'a str>;
        }
    }
}

#[cfg(test)]
pub use mocks::MockRootState;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use crate::{
        expr::v3::traits::{ExprText, ExprValue, OutputScope, WritableRuntimeExprContext},
        runner::v3::state::{ActionState, JobState, NodeState, RootState, State, StepState},
    };

    #[test]
    pub fn step_state_update_state_success() {
        let states = vec![
            State::Default,
            State::Running,
            State::Completed,
            State::Failed {
                error: "error".to_string(),
            },
        ];
        for state in states {
            let id = Uuid::new_v4().to_string();
            let expected = StepState {
                id: id.clone(),
                state: state.clone(),
                outputs: HashMap::new(),
            };
            let mut actual = StepState::new(&id);
            actual.update_state(state);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    pub fn step_state_get_exec_id_success() {
        let data = vec!["123", "hello", "world", "john", "doe"];
        for id in data {
            let state = StepState {
                id: id.to_string(),
                ..Default::default()
            };
            let exec_id = state.get_exec_id();
            assert!(exec_id.is_some());
            assert_eq!(id, exec_id.unwrap());
        }
    }

    #[test]
    pub fn step_state_get_output_success() {
        let outputs: HashMap<String, String> =
            vec![("name", "john"), ("surname", "doe"), ("age", "30")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        let id = Uuid::new_v4().to_string();
        let expr_outputs: HashMap<String, ExprValue<'static>> = outputs
            .iter()
            .map(|(k, v)| (k.clone(), ExprValue::Text(ExprText::Owned(v.clone()))))
            .collect();
        let state = StepState {
            id: id.clone(),
            state: State::Default,
            outputs: expr_outputs,
        };
        for (name, expected_value) in outputs {
            let actual_value = state.get_output(OutputScope::Step, &id, &name).unwrap();
            assert_eq!(
                actual_value,
                ExprValue::Text(ExprText::Owned(expected_value))
            );
        }
    }

    #[test]
    pub fn step_state_set_output_success() {
        let outputs: HashMap<String, String> =
            vec![("name", "john"), ("surname", "doe"), ("age", "30")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        let id = Uuid::new_v4().to_string();
        let mut state = StepState {
            id: id.clone(),
            state: State::Default,
            outputs: HashMap::new(),
        };
        for (name, value) in outputs {
            let result = state.set_output(&id, name, value);
            assert!(result.is_ok())
        }
    }

    #[test]
    pub fn step_state_set_outputs_success() {
        let outputs: HashMap<String, String> =
            vec![("name", "john"), ("surname", "doe"), ("age", "30")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        let id = Uuid::new_v4().to_string();
        let mut state = StepState {
            id: id.clone(),
            state: State::Default,
            outputs: HashMap::new(),
        };
        let result = state.set_outputs(&id, outputs);
        assert!(result.is_ok())
    }

    #[test]
    pub fn step_state_output_round_trip_preserves_the_original_text() {
        let data = vec![
            ("ver", "007"),
            ("dec", "1.10"),
            ("zero", "0.0"),
            ("exp", "1e3"),
            ("sha", "12345678901234567890123456789012"),
            ("nan", "NaN"),
            ("inf", "-inf"),
            ("name", "john"),
            ("flag", "true"),
        ];

        let id = Uuid::new_v4().to_string();
        let mut state = StepState {
            id: id.clone(),
            state: State::Default,
            outputs: HashMap::new(),
        };

        for (name, value) in &data {
            state
                .set_output(&id, name.to_string(), value.to_string())
                .unwrap();
        }

        for (name, value) in &data {
            let actual = state.get_output(OutputScope::Step, &id, name).unwrap();
            assert_eq!(
                actual.to_string(),
                *value,
                "output '{name}' was not preserved"
            );
        }
    }

    #[test]
    pub fn job_state_update_state_success() {
        let states = vec![
            State::Default,
            State::Running,
            State::Completed,
            State::Failed {
                error: "error".to_string(),
            },
        ];
        for state in states {
            let id = Uuid::new_v4().to_string();
            let expected = JobState {
                name: id.clone(),
                state: state.clone(),
                steps: HashMap::new(),
                matrix: HashMap::new(),
                job_outputs: HashMap::new(),
            };
            let mut actual = JobState::new(&id);
            actual.update_state(state);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    pub fn job_state_get_exec_id_success() {
        let data = vec!["123", "hello", "world", "john", "doe"];
        for id in data {
            let state = JobState {
                name: id.to_string(),
                ..Default::default()
            };
            let exec_id = state.get_exec_id();
            assert!(exec_id.is_some());
            assert_eq!(id, exec_id.unwrap());
        }
    }

    #[test]
    pub fn job_state_get_output_success() {
        let outputs: HashMap<String, String> =
            vec![("name", "john"), ("surname", "doe"), ("age", "30")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        let job_id = Uuid::new_v4().to_string();
        let step_id = Uuid::new_v4().to_string();
        let expr_outputs: HashMap<String, ExprValue<'static>> = outputs
            .iter()
            .map(|(k, v)| (k.clone(), ExprValue::Text(ExprText::Owned(v.clone()))))
            .collect();
        let mut state = JobState {
            name: job_id.clone(),
            state: State::Default,
            steps: HashMap::new(),
            matrix: HashMap::new(),
            job_outputs: HashMap::new(),
        };
        state.steps.insert(
            step_id.clone(),
            StepState {
                id: step_id.clone(),
                state: State::Default,
                outputs: expr_outputs,
            },
        );
        for (name, expected_value) in outputs {
            let actual_value = state
                .get_output(OutputScope::Step, &step_id, &name)
                .unwrap();
            assert_eq!(
                actual_value,
                ExprValue::Text(ExprText::Owned(expected_value))
            );
        }
    }

    #[test]
    pub fn job_state_set_output_success() {
        let outputs: HashMap<String, String> =
            vec![("name", "john"), ("surname", "doe"), ("age", "30")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        let job_id = Uuid::new_v4().to_string();
        let step_id = Uuid::new_v4().to_string();
        let expr_outputs: HashMap<String, ExprValue<'static>> = outputs
            .iter()
            .map(|(k, v)| (k.clone(), ExprValue::Text(ExprText::Owned(v.clone()))))
            .collect();
        let mut state = JobState {
            name: job_id.clone(),
            state: State::Default,
            steps: HashMap::new(),
            matrix: HashMap::new(),
            job_outputs: HashMap::new(),
        };
        state.steps.insert(
            step_id.clone(),
            StepState {
                id: step_id.clone(),
                state: State::Default,
                outputs: expr_outputs,
            },
        );
        for (name, value) in outputs {
            let result = state.set_output(&step_id, name, value);
            assert!(result.is_ok())
        }
    }

    #[test]
    pub fn job_state_set_outputs_success() {
        let outputs: HashMap<String, String> =
            vec![("name", "john"), ("surname", "doe"), ("age", "30")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        let job_id = Uuid::new_v4().to_string();
        let step_id = Uuid::new_v4().to_string();
        let expr_outputs: HashMap<String, ExprValue<'static>> = outputs
            .iter()
            .map(|(k, v)| (k.clone(), ExprValue::Text(ExprText::Owned(v.clone()))))
            .collect();
        let mut state = JobState {
            name: job_id.clone(),
            state: State::Default,
            steps: HashMap::new(),
            matrix: HashMap::new(),
            job_outputs: HashMap::new(),
        };
        state.steps.insert(
            step_id.clone(),
            StepState {
                id: step_id.clone(),
                state: State::Default,
                outputs: expr_outputs,
            },
        );
        let result = state.set_outputs(&step_id, outputs);
        assert!(result.is_ok())
    }

    #[test]
    pub fn job_state_get_job_output_success() {
        let mut state = JobState::new("consumer");
        let mut build_outputs = HashMap::new();
        build_outputs.insert("version".to_string(), "1.2.3".to_string());
        let mut job_outputs = HashMap::new();
        job_outputs.insert("build".to_string(), build_outputs);

        state.set_job_outputs(job_outputs).unwrap();

        let value = state
            .get_output(OutputScope::Job, "build", "version")
            .unwrap();
        assert_eq!(value, ExprValue::Text(ExprText::Owned("1.2.3".to_string())));
    }

    #[test]
    pub fn job_state_get_job_output_unknown_job_failure() {
        let state = JobState::new("consumer");
        let error = state
            .get_output(OutputScope::Job, "missing", "version")
            .unwrap_err();
        assert!(error.to_string().contains("missing"), "{error}");
    }

    #[test]
    pub fn job_state_get_job_output_unknown_output_failure() {
        let mut state = JobState::new("consumer");
        let mut job_outputs = HashMap::new();
        job_outputs.insert("build".to_string(), HashMap::new());

        state.set_job_outputs(job_outputs).unwrap();

        let error = state
            .get_output(OutputScope::Job, "build", "version")
            .unwrap_err();
        assert!(error.to_string().contains("version"), "{error}");
    }

    #[test]
    pub fn action_state_get_job_output_failure() {
        let state = ActionState::default();
        assert!(
            state
                .get_output(OutputScope::Job, "build", "version")
                .is_err()
        );
    }

    #[test]
    pub fn action_state_get_exec_id_success() {
        let data = vec!["123", "hello", "world", "john", "doe"];
        for id in data {
            let state = ActionState {
                id: id.to_string(),
                ..Default::default()
            };
            let exec_id = state.get_exec_id();
            assert!(exec_id.is_some());
            assert_eq!(id, exec_id.unwrap());
        }
    }

    #[test]
    pub fn action_state_get_output_success() {
        let outputs: HashMap<String, String> =
            vec![("name", "john"), ("surname", "doe"), ("age", "30")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        let action_id = Uuid::new_v4().to_string();
        let step_id = Uuid::new_v4().to_string();
        let expr_outputs: HashMap<String, ExprValue<'static>> = outputs
            .iter()
            .map(|(k, v)| (k.clone(), ExprValue::Text(ExprText::Owned(v.clone()))))
            .collect();
        let mut state = ActionState {
            id: action_id.clone(),
            state: State::Default,
            steps: HashMap::new(),
            matrix: HashMap::new(),
        };
        state.steps.insert(
            step_id.clone(),
            StepState {
                id: step_id.clone(),
                state: State::Default,
                outputs: expr_outputs,
            },
        );
        for (name, expected_value) in outputs {
            let actual_value = state
                .get_output(OutputScope::Step, &step_id, &name)
                .unwrap();
            assert_eq!(
                actual_value,
                ExprValue::Text(ExprText::Owned(expected_value))
            );
        }
    }

    #[test]
    pub fn action_state_set_output_success() {
        let outputs: HashMap<String, String> =
            vec![("name", "john"), ("surname", "doe"), ("age", "30")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        let action_id = Uuid::new_v4().to_string();
        let step_id = Uuid::new_v4().to_string();
        let expr_outputs: HashMap<String, ExprValue<'static>> = outputs
            .iter()
            .map(|(k, v)| (k.clone(), ExprValue::Text(ExprText::Owned(v.clone()))))
            .collect();
        let mut state = ActionState {
            id: action_id.clone(),
            state: State::Default,
            steps: HashMap::new(),
            matrix: HashMap::new(),
        };
        state.steps.insert(
            step_id.clone(),
            StepState {
                id: step_id.clone(),
                state: State::Default,
                outputs: expr_outputs,
            },
        );
        for (name, value) in outputs {
            let result = state.set_output(&step_id, name, value);
            assert!(result.is_ok())
        }
    }

    #[test]
    pub fn action_state_set_outputs_success() {
        let outputs: HashMap<String, String> =
            vec![("name", "john"), ("surname", "doe"), ("age", "30")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        let action_id = Uuid::new_v4().to_string();
        let step_id = Uuid::new_v4().to_string();
        let expr_outputs: HashMap<String, ExprValue<'static>> = outputs
            .iter()
            .map(|(k, v)| (k.clone(), ExprValue::Text(ExprText::Owned(v.clone()))))
            .collect();
        let mut state = ActionState {
            id: action_id.clone(),
            state: State::Default,
            steps: HashMap::new(),
            matrix: HashMap::new(),
        };
        state.steps.insert(
            step_id.clone(),
            StepState {
                id: step_id.clone(),
                state: State::Default,
                outputs: expr_outputs,
            },
        );
        let result = state.set_outputs(&step_id, outputs);
        assert!(result.is_ok())
    }
}
