use std::iter::Peekable;

use anyhow::{Result, anyhow, bail};
use bld_config::RegistryConfig;
use pest::iterators::Pairs;
use serde::{Deserialize, Serialize};

use crate::expr::v3::{
    parser::Rule,
    traits::{
        EvalObject, ExprText, ExprValue, ReadonlyRuntimeExprContext, WritableRuntimeExprContext,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Registry {
    FromConfig(String),
    Full(RegistryConfig),
}

#[cfg(feature = "all")]
impl<'a> EvalObject<'a> for Registry {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'_, Rule>>,
        _rctx: &'a RCtx,
        _wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>> {
        match self {
            Self::FromConfig(config) => {
                if path.peek().is_some() {
                    bail!("invalid expression for runs_on");
                }
                Ok(ExprValue::Text(ExprText::Ref(config)))
            }

            Self::Full(config) => {
                let Some(object) = path.next() else {
                    bail!("no object path present to evaluate runs_on");
                };

                let value = match object.as_span().as_str() {
                    "url" => config.url.as_str(),
                    "username" => config
                        .username
                        .as_ref()
                        .ok_or_else(|| anyhow!("no username value available for registry field"))?
                        .as_str(),
                    "password" => config
                        .password
                        .as_ref()
                        .ok_or_else(|| anyhow!("no username value available for registry field"))?
                        .as_str(),
                    value => bail!("invalid registry field: {value}"),
                };

                if path.peek().is_some() {
                    bail!("invalid expression for runs_on");
                }
                Ok(ExprValue::Text(ExprText::Ref(value)))
            }
        }
    }
}
