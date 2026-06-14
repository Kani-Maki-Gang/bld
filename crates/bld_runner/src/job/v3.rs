use crate::{runs_on::v3::RunsOn, step::v3::Step};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "all")]
use std::iter::Peekable;

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[cfg(feature = "all")]
use bld_core::fs::FileSystem;

#[cfg(feature = "all")]
use pest::iterators::Pairs;

#[cfg(feature = "all")]
use anyhow::Result;

#[cfg(feature = "all")]
use crate::{
    expr::v3::{
        parser::Rule,
        traits::{
            EvalObject, ExprValue, ReadonlyRuntimeExprContext, WritableRuntimeExprContext,
        },
    },
    traits::Dependencies,
    validator::v3::{Validate, ValidatorContext},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JobNeeds {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    #[serde(default = "Job::default_id")]
    pub id: String,
    pub name: Option<String>,
    pub runs_on: RunsOn,
    #[serde(rename = "if")]
    pub condition: Option<String>,
    pub needs: Option<JobNeeds>,
    pub steps: Vec<Step>,
}

impl Job {
    fn default_id() -> String {
        Uuid::new_v4().to_string()
    }
}

impl Default for Job {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            name: None,
            runs_on: RunsOn::default(),
            condition: None,
            needs: None,
            steps: vec![],
        }
    }
}

#[cfg(feature = "all")]
impl Dependencies for Job {
    async fn local_deps(&self, _config: &BldConfig, _fs: &FileSystem) -> Vec<String> {
        unimplemented!()
    }
}

#[cfg(feature = "all")]
impl<'a> EvalObject<'a> for Step {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        _path: &mut Peekable<Pairs<'_, Rule>>,
        _rctx: &'a RCtx,
        _wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>> {
        unimplemented!()
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Step {
    async fn validate<C: ValidatorContext<'a>>(&'a self, _ctx: &mut C) {
        unimplemented!()
    }
}
