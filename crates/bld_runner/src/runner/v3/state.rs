use std::collections::HashMap;

use anyhow::{Result, anyhow, bail};
use mockall::{automock, mock};
use uuid::Uuid;

use crate::expr::v3::traits::WritableRuntimeExprContext;

#[automock]
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
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum State {
    #[default]
    Default,
    Running,
    Completed,
    Failed {
        error: String,
    },
}

#[derive(Debug, Default, PartialEq)]
pub struct StepState {
    id: String,
    state: State,
    outputs: HashMap<String, String>,
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

    fn get_output<'a>(&'a self, id: &str, name: &str) -> Result<&'a str> {
        if self.id != id {
            bail!("id {id} has no outputs");
        }
        self.outputs
            .get(name)
            .map(|x| x.as_str())
            .ok_or_else(|| anyhow!("output '{name}' not found"))
    }

    fn set_output(&mut self, id: &str, name: String, value: String) -> Result<()> {
        if self.id != id {
            bail!("target id {id} is inaccessible");
        }
        let _ = self.outputs.insert(name, value);
        Ok(())
    }

    fn set_outputs(&mut self, id: &str, outputs: HashMap<String, String>) -> Result<()> {
        if self.id != id {
            bail!("target id {id} is inaccessible");
        }
        self.outputs = outputs;
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct JobState {
    id: String,
    state: State,
    steps: HashMap<String, StepState>,
}

impl JobState {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            ..Default::default()
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
}

impl WritableRuntimeExprContext for JobState {
    fn get_exec_id(&self) -> Option<&str> {
        Some(self.id.as_str())
    }

    fn get_output<'a>(&'a self, id: &str, name: &str) -> Result<&'a str> {
        let Some(step_state) = self.steps.get(id) else {
            bail!("outputs for id {id} weren't found");
        };
        step_state.get_output(id, name)
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
}

pub struct ActionState {
    id: String,
    state: State,
    steps: HashMap<String, StepState>,
}

impl Default for ActionState {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            state: State::Default,
            steps: HashMap::new(),
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
}

impl WritableRuntimeExprContext for ActionState {
    fn get_exec_id(&self) -> Option<&str> {
        Some(self.id.as_str())
    }

    fn get_output<'a>(&'a self, id: &str, name: &str) -> Result<&'a str> {
        let Some(step_state) = self.steps.get(id) else {
            bail!("outputs for id {id} weren't found");
        };
        step_state.get_output(id, name)
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
}

mock! {
    pub RootState {}

    impl RootState for RootState {
        fn update_state(&mut self, state: State);
        fn get_state(&self) -> &State;
        fn add_node(&mut self, node_id: &str);
        fn update_node_state(&mut self, node_id: &str, state: State);
        fn get_node_state<'a>(&'a self, node_id: &str) -> Option<&'a State>;
    }

    impl WritableRuntimeExprContext for RootState {
        fn get_exec_id<'a> (&'a self) -> Option<&'a str>;
        fn get_output<'a>(&'a self, id: &str, name: &str) -> Result<&'a str>;
        fn set_output(&mut self, id: &str, name: String, value: String) -> Result<()>;
        fn set_outputs(&mut self, id: &str, outputs: HashMap<String, String>) -> Result<()>;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use crate::{
        expr::v3::traits::WritableRuntimeExprContext,
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
        let state = StepState {
            id: id.clone(),
            state: State::Default,
            outputs: outputs.clone(),
        };
        for (name, expected_value) in outputs {
            let actual_value = state.get_output(&id, &name).unwrap();
            assert_eq!(actual_value, expected_value);
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
                id: id.clone(),
                state: state.clone(),
                steps: HashMap::new(),
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
                id: id.to_string(),
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
        let mut state = JobState {
            id: job_id.clone(),
            state: State::Default,
            steps: HashMap::new(),
        };
        state.steps.insert(
            step_id.clone(),
            StepState {
                id: step_id.clone(),
                state: State::Default,
                outputs: outputs.clone(),
            },
        );
        for (name, expected_value) in outputs {
            let actual_value = state.get_output(&step_id, &name).unwrap();
            assert_eq!(actual_value, expected_value);
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
        let mut state = JobState {
            id: job_id.clone(),
            state: State::Default,
            steps: HashMap::new(),
        };
        state.steps.insert(
            step_id.clone(),
            StepState {
                id: step_id.clone(),
                state: State::Default,
                outputs: outputs.clone(),
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
        let mut state = JobState {
            id: job_id.clone(),
            state: State::Default,
            steps: HashMap::new(),
        };
        state.steps.insert(
            step_id.clone(),
            StepState {
                id: step_id.clone(),
                state: State::Default,
                outputs: outputs.clone(),
            },
        );
        let result = state.set_outputs(&step_id, outputs);
        assert!(result.is_ok())
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
        let mut state = ActionState {
            id: action_id.clone(),
            state: State::Default,
            steps: HashMap::new(),
        };
        state.steps.insert(
            step_id.clone(),
            StepState {
                id: step_id.clone(),
                state: State::Default,
                outputs: outputs.clone(),
            },
        );
        for (name, expected_value) in outputs {
            let actual_value = state.get_output(&step_id, &name).unwrap();
            assert_eq!(actual_value, expected_value);
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
        let mut state = ActionState {
            id: action_id.clone(),
            state: State::Default,
            steps: HashMap::new(),
        };
        state.steps.insert(
            step_id.clone(),
            StepState {
                id: step_id.clone(),
                state: State::Default,
                outputs: outputs.clone(),
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
        let mut state = ActionState {
            id: action_id.clone(),
            state: State::Default,
            steps: HashMap::new(),
        };
        state.steps.insert(
            step_id.clone(),
            StepState {
                id: step_id.clone(),
                state: State::Default,
                outputs: outputs.clone(),
            },
        );
        let result = state.set_outputs(&step_id, outputs);
        assert!(result.is_ok())
    }
}
