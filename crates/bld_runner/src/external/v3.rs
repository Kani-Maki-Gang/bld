use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "all")]
use crate::{
    Load, Yaml,
    validator::v3::{Validate, ValidatorContext},
};

#[cfg(feature = "all")]
use bld_utils::fs::IsYaml;

#[cfg(feature = "all")]
use tracing::debug;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct External {
    pub name: Option<String>,
    pub server: Option<String>,
    pub uses: String,

    #[serde(default)]
    pub with: HashMap<String, String>,

    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl External {
    pub fn is(&self, value: &str) -> bool {
        self.name.as_ref().map(|n| n == value).unwrap_or_default() || self.uses == value
    }

    pub fn local(uses: &str) -> Self {
        Self {
            uses: uses.to_owned(),
            ..Default::default()
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for External {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        if let Some(name) = self.name.as_deref() {
            debug!("Validating external's name value");
            ctx.push_section("name");
            ctx.validate_symbols(name);
            ctx.pop_section();
        };

        debug!("Validating external's uses value");
        ctx.push_section("uses");
        validate_external_file(ctx, &self.uses).await;
        ctx.pop_section();

        debug!("Validating external's server value");
        ctx.push_section("server");
        validate_external_server(ctx, self.server.as_deref());
        ctx.pop_section();

        debug!("Validating external's with section");
        ctx.push_section("with");
        ctx.push_section("inputs");
        validate_external_with(ctx, &self.uses, self.server.as_deref(), &self.with).await;
        ctx.pop_section();

        debug!("Validating external's env section");
        ctx.push_section("env");
        ctx.validate_env(&self.env);
        ctx.pop_section();
    }
}

#[cfg(feature = "all")]
async fn validate_external_file<'a, C: ValidatorContext<'a>>(ctx: &mut C, uses: &'a str) {
    if ctx.contains_symbols(uses) {
        ctx.validate_symbols(uses);
    } else {
        let fs = ctx.get_fs();
        match fs.path(uses).await {
            Ok(path) if !path.is_yaml() => {
                ctx.push_section(uses);
                ctx.append_error("Pipeline or action not found");
                ctx.pop_section();
            }
            Err(e) => {
                ctx.push_section(uses);
                ctx.append_error(&e.to_string());
                ctx.pop_section();
            }
            _ => {}
        }
    }
}

#[cfg(feature = "all")]
fn validate_external_server<'a, C: ValidatorContext<'a>>(ctx: &mut C, server: Option<&'a str>) {
    let Some(server) = server else {
        return;
    };

    if ctx.contains_symbols(server) {
        ctx.validate_symbols(server);
    } else {
        let config = ctx.get_config();
        if config.server(server).is_err() {
            ctx.push_section(server);
            ctx.append_error("Doesn't exist in current config");
            ctx.pop_section();
        }
    }
}

#[cfg(feature = "all")]
async fn validate_external_with<'a, C: ValidatorContext<'a>>(
    ctx: &mut C,
    uses: &'a str,
    server: Option<&'a str>,
    with: &'a HashMap<String, String>,
) {
    if server.is_none() {
        let fs = ctx.get_fs();
        let file = fs
            .read(uses)
            .await
            .and_then(|c| Yaml::load_with_verbose_errors(&c));

        match file {
            Ok(file) => {
                let required = file.required_inputs();
                if let Some(required) = required {
                    for name in required {
                        if !with.contains_key(name) {
                            let message = format!("Missing required input: {}", name);
                            ctx.append_error(&message);
                        }
                    }
                }
            }
            Err(e) => {
                let message = format!("Unable to check required inputs due to {e}");
                ctx.append_error(&message);
            }
        }
    }

    for (name, input) in with.iter() {
        debug!("Validating input: {}", name);
        ctx.push_section(name);
        ctx.validate_keywords(name);
        ctx.validate_symbols(input);
        ctx.pop_section();
    }
}
