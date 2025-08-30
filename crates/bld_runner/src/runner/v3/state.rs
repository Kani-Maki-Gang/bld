use std::collections::HashMap;

use anyhow::{Result, anyhow, bail};
use uuid::Uuid;

use crate::expr::v3::traits::WritableRuntimeExprContext;

pub enum State {
    Default,
    Running,
    Completed,
    Failed { error: String },
}

impl Default for State {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Default)]
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

    pub fn update_state(&mut self, state: State) {
        self.state = state;
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

#[derive(Default)]
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

    pub fn update_state(&mut self, state: State) {
        self.state = state;
    }

    pub fn add_step(&mut self, step_id: &str) {
        self.steps
            .insert(step_id.to_string(), StepState::new(step_id));
    }

    pub fn update_step_state(&mut self, step_id: &str, state: State) {
        let Some(step_state) = self.steps.get_mut(step_id) else {
            return;
        };
        step_state.update_state(state);
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

impl ActionState {
    pub fn update_state(&mut self, state: State) {
        self.state = state;
    }

    pub fn add_step(&mut self, step_id: &str) {
        self.steps
            .insert(step_id.to_string(), StepState::new(step_id));
    }

    pub fn update_step_state(&mut self, step_id: &str, state: State) {
        let Some(step_state) = self.steps.get_mut(step_id) else {
            return;
        };
        step_state.update_state(state);
    }
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
