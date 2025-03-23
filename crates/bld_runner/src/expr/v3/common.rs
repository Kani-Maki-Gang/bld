use std::collections::HashMap;
use anyhow::{anyhow, Result};
use super::traits::RuntimeExecutionContext;

#[derive(Debug, Default)]
pub struct CommonRuntimeExecutionContext<'a> {
    root_dir: &'a str,
    project_dir: &'a str,
    inputs: HashMap<String, String>,
    outputs: HashMap<&'a str, &'a str>,
    env: HashMap<String, String>,
    run_id: &'a str,
    run_start_time: &'a str,
}

impl<'a> RuntimeExecutionContext<'a> for CommonRuntimeExecutionContext<'a> {
    fn get_root_dir(&self) -> &'a str {
        self.root_dir
    }

    fn get_project_dir(&self) -> &'a str {
        self.project_dir
    }

    fn get_input(&'a self, name: &'a str) -> Result<&'a str> {
        self.inputs
            .get(name)
            .map(|x| x.as_str())
            .ok_or_else(|| anyhow!("input '{name}' not found"))
    }

    fn get_output(&self, name: &'a str) -> Result<&'a str> {
        self.outputs
            .get(name)
            .copied()
            .ok_or_else(|| anyhow!("output '{name}' not found"))
    }

    fn set_output(&mut self, name: &'a str, value: &'a str) -> Result<()> {
        self.outputs.insert(name, value);
        Ok(())
    }

    fn get_env(&'a self, name: &'a str) -> Result<&'a str> {
        self.env
            .get(name)
            .map(|x| x.as_str())
            .ok_or_else(|| anyhow!("env variable '{name}' not found"))
    }

    fn get_run_id(&self) -> &'a str {
        self.run_id
    }

    fn get_run_start_time(&self) -> &'a str {
        self.run_start_time
    }
}

// pub struct CommonExprExecutor<'a, R, S>
// where
//     R: RuntimeExecutionContext<'a>,
//     S: StaticExecutionContext<'a>,
// {
//     runtime_ctx: &'a R,
//     object_evaluator: &'a S,
// }
//
// impl<'a, R, S> CommonExprExecutor<'a, R, S>
// where
//     R: RuntimeExecutionContext<'a>,
//     S: StaticExecutionContext<'a>,
// {
//     pub fn new(runtime_ctx: &'a R, static_ctx: &'a S) -> Self {
//         Self {
//             runtime_ctx,
//             static_ctx,
//         }
//     }
// }
//
// impl<'a, R, S> ExprEvaluate for CommonExprExecutor<'a, R, S>
// where
//     R: RuntimeExecutionContext<'a>,
//     S: StaticExecutionContext<'a>,
// {
//     fn eval(&mut self, expr: &str) -> Result<ExprValue> {
//         let pairs = ExprParser::parse(Rule::Full, expr)?;
//         unimplemented!()
//     }
// }
