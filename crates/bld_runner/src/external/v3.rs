use bld_utils::fs::IsYaml;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "all")]
use anyhow::Result;

#[cfg(feature = "all")]
use crate::token_context::v3::ExecutionContext;
use crate::validator::v3::{Validatable, ValidatorContext};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct External {
    pub name: Option<String>,
    pub server: Option<String>,
    pub pipeline: String,

    #[serde(default)]
    pub inputs: HashMap<String, String>,

    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl External {
    pub fn is(&self, value: &str) -> bool {
        self.name.as_ref().map(|n| n == value).unwrap_or_default() || self.pipeline == value
    }

    pub fn local(pipeline: &str) -> Self {
        Self {
            pipeline: pipeline.to_owned(),
            ..Default::default()
        }
    }

    #[cfg(feature = "all")]
    pub async fn apply_tokens<'a>(&mut self, context: &'a ExecutionContext<'a>) -> Result<()> {
        if let Some(name) = self.name.as_mut() {
            *name = context.transform(name.to_owned()).await?;
        }

        if let Some(server) = self.server.as_mut() {
            *server = context.transform(server.to_owned()).await?;
        }

        self.pipeline = context.transform(self.pipeline.to_owned()).await?;

        for (_, v) in self.inputs.iter_mut() {
            *v = context.transform(v.to_owned()).await?;
        }

        for (_, v) in self.env.iter_mut() {
            *v = context.transform(v.to_owned()).await?;
        }

        Ok(())
    }
}

impl<'a> Validatable<'a> for External {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        if let Some(name) = self.name.as_deref() {
            ctx.push_section("name");
            ctx.validate_symbols(name);
            ctx.pop_section();
        };
        ctx.push_section("pipeline");
        validate_external_pipeline(ctx, &self.pipeline).await;
        ctx.pop_section();

        ctx.push_section("server");
        validate_external_server(ctx, self.server.as_deref());
        ctx.pop_section();

        ctx.validate_inputs(&self.inputs);
        ctx.validate_env(&self.env);
    }
}

async fn validate_external_pipeline<'a, C: ValidatorContext<'a>>(ctx: &mut C, pipeline: &'a str) {
    ctx.validate_symbols(pipeline);

    if ctx.contains_symbols(pipeline) {
        return;
    }

    let fs = ctx.get_fs();
    match fs.path(pipeline).await {
        Ok(path) if !path.is_yaml() => {
            ctx.push_section(pipeline);
            ctx.append_error("Pipeline not found");
            ctx.pop_section();
        }
        Err(e) => {
            ctx.push_section(pipeline);
            ctx.append_error(&e.to_string());
            ctx.pop_section();
        }
        _ => {}
    }
}

fn validate_external_server<'a, C: ValidatorContext<'a>>(ctx: &mut C, server: Option<&'a str>) {
    let Some(server) = server else {
        return;
    };

    ctx.validate_symbols(server);

    if ctx.contains_symbols(server) {
        return;
    }

    let config = ctx.get_config();
    if config.server(server).is_err() {
        ctx.push_section(server);
        ctx.append_error("Doesn't exist in current config");
        ctx.pop_section();
    }
}
