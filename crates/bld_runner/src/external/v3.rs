use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(feature = "all")]
use {
    crate::{
        expr::v3::{
            parser::Rule,
            traits::{
                EvalObject, ExprText, ExprValue, ReadonlyRuntimeExprContext,
                WritableRuntimeExprContext,
            },
        },
        validator::v3::{Validate, ValidatorContext},
    },
    anyhow::{Result, bail},
    pest::iterators::Pairs,
    std::iter::Peekable,
    tracing::debug,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct External {
    #[serde(default = "External::default_id")]
    pub id: String,
    pub name: Option<String>,
    pub server: Option<String>,
    pub uses: String,

    #[serde(default)]
    pub with: HashMap<String, String>,

    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl External {
    fn default_id() -> String {
        Uuid::new_v4().to_string()
    }

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
impl<'a> EvalObject<'a> for External {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'_, Rule>>,
        _rctx: &'a RCtx,
        _wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>> {
        let Some(object) = path.next() else {
            bail!("no object path present");
        };

        let key = object.as_span().as_str();

        let value = match key {
            "name" => self.name.as_deref().unwrap_or(""),
            "server" => self.server.as_deref().unwrap_or(""),
            "uses" => &self.uses,
            "with" => {
                let Some(key) = path.next() else {
                    bail!("expected object key for with field");
                };
                let key = key.as_span().as_str();
                let Some(value) = self.with.get(key) else {
                    bail!("object key '{}' not found in with field", key);
                };
                value
            }
            "env" => {
                let Some(key) = path.next() else {
                    bail!("expected object key for env field");
                };
                let key = key.as_span().as_str();
                let Some(value) = self.env.get(key) else {
                    bail!("object key '{}' not found in env field", key);
                };
                value
            }
            value => bail!("invalid steps field: {value}"),
        };

        Ok(ExprValue::Text(ExprText::Ref(value)))
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for External {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        if let Some(name) = self.name.as_deref() {
            debug!("Validating external's name value");
            ctx.push_section("name");
            ctx.validate_expressions(name);
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
    use crate::VersionedFileLoader;

    if ctx.contains_expressions(uses) {
        ctx.validate_expressions(uses);
        return;
    }

    let file_system = ctx.get_fs();
    let package_manager = ctx.get_package_manager();
    let loader = VersionedFileLoader::new(&package_manager, &file_system, true);
    if loader.get_source(uses).await.is_none() {
        ctx.push_section(uses);
        ctx.append_error("Pipeline or action not found");
        ctx.pop_section();
    }
}

#[cfg(feature = "all")]
fn validate_external_server<'a, C: ValidatorContext<'a>>(ctx: &mut C, server: Option<&'a str>) {
    let Some(server) = server else {
        return;
    };

    if ctx.contains_expressions(server) {
        ctx.validate_expressions(server);
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
    use crate::VersionedFileLoader;

    if server.is_none() {
        let file_system = ctx.get_fs();
        let package_manager = ctx.get_package_manager();
        let loader = VersionedFileLoader::new(&package_manager, &file_system, true);
        match loader.load(uses).await {
            Ok(metadata) => {
                let required = metadata.file.required_inputs();
                if let Some(required) = required {
                    for name in required {
                        if !with.contains_key(name) {
                            let message = format!("Missing required input: {name}");
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
        ctx.validate_expressions(input);
        ctx.pop_section();
    }
}
