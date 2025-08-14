use crate::registry::v3::Registry;
use bld_config::SshConfig;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[cfg(feature = "all")]
use std::iter::Peekable;

#[cfg(feature = "all")]
use anyhow::{anyhow, bail};

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
    anyhow::Result,
    bld_config::{DockerUrl, SshUserAuth},
    pest::iterators::Pairs,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunsOn {
    ContainerOrMachine(String),
    Pull {
        image: String,
        registry: Option<Registry>,
        pull: Option<bool>,
        docker_url: Option<String>,
    },
    Build {
        name: String,
        tag: String,
        dockerfile: String,
        docker_url: Option<String>,
    },
    Ssh(SshConfig),
    SshFromGlobalConfig {
        ssh_config: String,
    },
}

impl Default for RunsOn {
    fn default() -> Self {
        Self::ContainerOrMachine("machine".to_string())
    }
}

impl Display for RunsOn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContainerOrMachine(image) if image == "machine" => write!(f, "machine"),
            Self::ContainerOrMachine(image) => write!(f, "{image}"),
            Self::Pull { image, .. } => write!(f, "{image}"),
            Self::Build { name, tag, .. } => write!(f, "{name}:{tag}"),
            Self::SshFromGlobalConfig { ssh_config } => write!(f, "{ssh_config}"),
            Self::Ssh(config) => write!(f, "{}:{}", config.host, config.port),
        }
    }
}

impl RunsOn {
    pub fn registry(&self) -> Option<&str> {
        match self {
            RunsOn::Pull {
                registry: Some(Registry::FromConfig(config)),
                ..
            } => Some(config),
            RunsOn::Pull {
                registry: Some(Registry::Full(config)),
                ..
            } => Some(&config.url),
            _ => None,
        }
    }

    pub fn registry_username(&self) -> Option<&str> {
        match self {
            RunsOn::Pull {
                registry: Some(Registry::Full(config)),
                ..
            } => config.username.as_deref(),
            _ => None,
        }
    }
}

#[cfg(feature = "all")]
impl<'a> EvalObject<'a> for RunsOn {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'a, Rule>>,
        rctx: &'a RCtx,
        wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>> {
        let value = match self {
            Self::ContainerOrMachine(value) => {
                if path.peek().is_some() {
                    bail!("invalid expression for runs_on expression");
                }
                ExprValue::Text(ExprText::Ref(value.as_str()))
            }

            Self::Pull {
                image,
                registry,
                pull,
                docker_url,
            } => {
                let Some(next) = path.next() else {
                    bail!("expected a path for evaluating runs_on",);
                };

                match next.as_span().as_str() {
                    "image" => ExprValue::Text(ExprText::Ref(image.as_str())),
                    "registry" => {
                        let registry = registry
                            .as_ref()
                            .ok_or_else(|| anyhow!("registry field is not set"))?;
                        return registry.eval_object(path, rctx, wctx);
                    }
                    "pull" => ExprValue::Boolean(pull.unwrap_or_default()),
                    "docker_url" => ExprValue::Text(ExprText::Ref(
                        docker_url.as_ref().map(|x| x.as_str()).unwrap_or_default(),
                    )),
                    value => bail!("invalid runs_on field: {value}"),
                }
            }

            Self::Build {
                name,
                tag,
                dockerfile,
                docker_url,
            } => {
                let Some(next) = path.next() else {
                    bail!("expected a path for evaluating runs_on",);
                };

                match next.as_span().as_str() {
                    "name" => ExprValue::Text(ExprText::Ref(name)),
                    "tag" => ExprValue::Text(ExprText::Ref(tag)),
                    "dockerfile" => ExprValue::Text(ExprText::Ref(dockerfile)),
                    "docker_url" => {
                        let docker_url = docker_url
                            .as_ref()
                            .ok_or_else(|| anyhow!("docker_url field is not set"))?;
                        ExprValue::Text(ExprText::Ref(docker_url))
                    }
                    value => bail!("invalid runs_on field: {value}"),
                }
            }

            Self::Ssh(config) => config.eval_object(path, rctx, wctx)?,

            Self::SshFromGlobalConfig { ssh_config } => {
                let Some(next) = path.next() else {
                    bail!("expected a path for evaluating runs_on",);
                };
                match next.as_span().as_str() {
                    "ssh_config" => ExprValue::Text(ExprText::Ref(ssh_config)),
                    value => bail!("invalid runs_on field: {value}"),
                }
            }
        };

        if path.peek().is_some() {
            bail!("invalid expression for runs_on");
        }

        Ok(value)
    }
}

#[cfg(feature = "all")]
impl<'a> EvalObject<'a> for SshConfig {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'a, Rule>>,
        rctx: &'a RCtx,
        wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>> {
        let Some(object) = path.next() else {
            bail!("no object path present to evaluate runs_on");
        };

        let value = match object.as_span().as_str() {
            "host" => &self.host,
            "port" => &self.port,
            "user" => &self.user,
            "userauth" => {
                return self.userauth.eval_object(path, rctx, wctx);
            }
            value => bail!("invalid runs_on field: {value}"),
        };

        if path.peek().is_some() {
            bail!("invalid expression for runs_on");
        }

        Ok(ExprValue::Text(ExprText::Ref(value)))
    }
}

#[cfg(feature = "all")]
impl<'a> EvalObject<'a> for SshUserAuth {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'a, Rule>>,
        _rctx: &'a RCtx,
        _wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>> {
        match self {
            Self::Keys {
                public_key,
                private_key,
            } => {
                let Some(object) = path.next() else {
                    bail!("no object path present to evaluate runs_on");
                };

                let value = match object.as_span().as_str() {
                    "type" => "keys",
                    "public_key" => public_key.as_ref().map(|x| x.as_str()).unwrap_or(""),
                    "private_key" => private_key.as_str(),
                    value => bail!("invalid userauth field: {value}"),
                };

                if path.peek().is_some() {
                    bail!("invalid expression for runs_on");
                }

                Ok(ExprValue::Text(ExprText::Ref(value)))
            }

            Self::Password { password } => {
                let Some(object) = path.next() else {
                    bail!("no object path present to evaluate runs_on");
                };

                let value = match object.as_span().as_str() {
                    "type" => "password",
                    "password" => password,
                    value => bail!("invalid userauth field: {value}"),
                };

                if path.peek().is_some() {
                    bail!("invalid expression for runs_on");
                }

                Ok(ExprValue::Text(ExprText::Ref(value)))
            }

            Self::Agent => {
                let Some(object) = path.next() else {
                    bail!("invalid expression for runs_on");
                };

                let value = match object.as_span().as_str() {
                    "type" => "agent",
                    value => bail!("invalid userauth field: {value}"),
                };

                if path.peek().is_some() {
                    bail!("invalid expression for runs_on");
                }

                Ok(ExprValue::Text(ExprText::Ref(value)))
            }
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for RunsOn {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        match &self {
            RunsOn::Build {
                name,
                tag,
                dockerfile,
                docker_url,
            } => {
                ctx.push_section("name");
                ctx.validate_symbols(name);
                ctx.pop_section();

                ctx.push_section("tag");
                ctx.validate_symbols(tag);
                ctx.pop_section();

                ctx.push_section("dockerfile");
                ctx.validate_symbols(dockerfile);
                ctx.validate_file_path(dockerfile);
                ctx.pop_section();

                if let Some(docker_url) = docker_url {
                    validate_docker_url(ctx, docker_url);
                }
            }

            RunsOn::Pull {
                image,
                docker_url,
                pull: _pull,
                registry,
            } => {
                ctx.push_section("image");
                ctx.validate_symbols(image);
                ctx.pop_section();

                if let Some(docker_url) = docker_url {
                    validate_docker_url(ctx, docker_url);
                }
                if let Some(registry) = registry.as_ref() {
                    validate_registry(ctx, registry);
                }
            }

            RunsOn::ContainerOrMachine(value) => ctx.validate_symbols(value),

            RunsOn::SshFromGlobalConfig { ssh_config } => {
                validate_global_ssh_config(ctx, ssh_config);
            }

            RunsOn::Ssh(config) => {
                ctx.push_section("host");
                ctx.validate_symbols(&config.host);
                ctx.pop_section();

                ctx.push_section("port");
                ctx.validate_symbols(&config.port);
                ctx.pop_section();

                ctx.push_section("user");
                ctx.validate_symbols(&config.user);
                ctx.pop_section();

                ctx.push_section("auth");
                match &config.userauth {
                    SshUserAuth::Agent => {}

                    SshUserAuth::Keys {
                        public_key,
                        private_key,
                    } => {
                        if let Some(pubkey) = public_key {
                            ctx.push_section("public_key");
                            ctx.validate_symbols(pubkey);
                            ctx.validate_file_path(pubkey);
                            ctx.pop_section();
                        }

                        ctx.push_section("private_key");
                        ctx.validate_symbols(private_key);
                        ctx.validate_file_path(private_key);
                        ctx.pop_section();
                    }

                    SshUserAuth::Password { password } => {
                        ctx.push_section("password");
                        ctx.validate_symbols(password);
                        ctx.pop_section();
                    }
                }
                ctx.pop_section();
            }
        }
    }
}

#[cfg(feature = "all")]
fn validate_docker_url<'a, C: ValidatorContext<'a>>(ctx: &mut C, value: &'a str) {
    ctx.push_section("docker_url");

    if ctx.contains_symbols(value) {
        ctx.validate_symbols(value);
    } else {
        let config = ctx.get_config();
        match &config.local.docker_url {
            DockerUrl::Single(_) => {
                ctx.append_error("Only a single docker url is defined in the config file");
            }
            DockerUrl::Multiple(urls) => {
                let url = urls.keys().find(|x| x.as_str() == value);
                if url.is_none() {
                    ctx.append_error("The defined docker url key wasn't found in the config file");
                }
            }
        }
    }

    ctx.pop_section();
}

#[cfg(feature = "all")]
fn validate_registry<'a, C: ValidatorContext<'a>>(ctx: &mut C, registry: &'a Registry) {
    ctx.push_section("registry");

    match registry {
        Registry::FromConfig(config) => {
            validate_global_registry_config(ctx, config);
        }
        Registry::Full(config) => {
            ctx.push_section("url");
            ctx.validate_symbols(&config.url);
            ctx.pop_section();

            if let Some(username) = &config.username {
                ctx.push_section("username");
                ctx.validate_symbols(username);
                ctx.pop_section();
            }

            if let Some(password) = &config.password {
                ctx.push_section("password");
                ctx.validate_symbols(password);
                ctx.pop_section();
            }
        }
    }

    ctx.pop_section();
}

#[cfg(feature = "all")]
fn validate_global_registry_config<'a, C: ValidatorContext<'a>>(ctx: &mut C, value: &'a str) {
    if ctx.contains_symbols(value) {
        ctx.validate_symbols(value);
    } else {
        let config = ctx.get_config();
        if config.registry(value).is_none() {
            ctx.append_error("The defined registry key wasn't found in the config file");
        }
    }
}

#[cfg(feature = "all")]
fn validate_global_ssh_config<'a, C: ValidatorContext<'a>>(ctx: &mut C, value: &'a str) {
    ctx.push_section("ssh_config");

    if ctx.contains_symbols(value) {
        ctx.validate_symbols(value);
    } else {
        let config = ctx.get_config();
        if let Err(e) = config.ssh(value) {
            ctx.append_error(&e.to_string());
        }
    }

    ctx.pop_section();
}

#[cfg(test)]
mod tests {
    use bld_config::{SshConfig, SshUserAuth};

    use crate::{
        expr::v3::{
            context::{CommonReadonlyRuntimeExprContext, CommonWritableRuntimeExprContext},
            exec::CommonExprExecutor,
            traits::{EvalExpr, ExprText, ExprValue},
        },
        pipeline::v3::Pipeline,
        registry::v3::Registry,
    };

    use super::RunsOn;

    #[test]
    pub fn runs_on_machine_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.runs_on = RunsOn::ContainerOrMachine("machine".to_string());
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        let expected = ExprValue::Text(ExprText::Ref("machine"));
        let actual = exec.eval("${{ runs_on }}").unwrap();
        assert!(matches!(
            actual.try_eq(&expected),
            Ok(ExprValue::Boolean(true))
        ));
    }

    #[test]
    pub fn runs_on_container_name_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();

        let data: Vec<(&str, ExprValue)> = vec![
            ("ubuntu", ExprValue::Text(ExprText::Ref("ubuntu"))),
            (
                "ubuntu:latest",
                ExprValue::Text(ExprText::Ref("ubuntu:latest")),
            ),
            (
                "ubuntu:24.04",
                ExprValue::Text(ExprText::Ref("ubuntu:24.04")),
            ),
            ("arch", ExprValue::Text(ExprText::Ref("arch"))),
            ("arch:latest", ExprValue::Text(ExprText::Ref("arch:latest"))),
        ];

        for (value, expected) in data {
            pipeline.runs_on = RunsOn::ContainerOrMachine(value.to_string());
            let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

            let actual = exec.eval("${{ runs_on }}").unwrap();
            assert!(matches!(
                actual.try_eq(&expected),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }

    #[test]
    pub fn runs_on_pull_image_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.runs_on = RunsOn::Pull {
            image: "ubuntu:latest".to_string(),
            registry: Some(Registry::FromConfig("registry-config".to_string())),
            pull: Some(true),
            docker_url: Some("docker-url".to_string()),
        };
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        let actual = exec.eval("${{ runs_on.image }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("ubuntu:latest"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.registry }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("registry-config"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.pull }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Boolean(true)),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.docker_url }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("docker-url"))),
            Ok(ExprValue::Boolean(true))
        ));
    }

    #[test]
    pub fn runs_on_build_image_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.runs_on = RunsOn::Build {
            name: "test-image".to_string(),
            tag: "1.3.4".to_string(),
            dockerfile: "path-to-dockerfile".to_string(),
            docker_url: Some("docker-url".to_string()),
        };
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        let actual = exec.eval("${{ runs_on.name}}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("test-image"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.tag }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("1.3.4"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.dockerfile }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("path-to-dockerfile"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.docker_url }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("docker-url"))),
            Ok(ExprValue::Boolean(true))
        ));
    }

    #[test]
    pub fn runs_on_ssh_with_user_auth_key_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.runs_on = RunsOn::Ssh(SshConfig {
            host: "localhost".to_string(),
            port: "3000".to_string(),
            user: "some_user".to_string(),
            userauth: SshUserAuth::Keys {
                public_key: Some("some_public_key".to_string()),
                private_key: "some_private_key".to_string(),
            },
        });
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        let actual = exec.eval("${{ runs_on.host }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("localhost"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.port }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("3000"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.user }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("some_user"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.userauth.type }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("keys"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.userauth.public_key }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("some_public_key"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.userauth.private_key }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("some_private_key"))),
            Ok(ExprValue::Boolean(true))
        ));
    }

    #[test]
    pub fn runs_on_ssh_with_user_password_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.runs_on = RunsOn::Ssh(SshConfig {
            host: "localhost".to_string(),
            port: "3000".to_string(),
            user: "some_user".to_string(),
            userauth: SshUserAuth::Password {
                password: "some_password".to_string(),
            },
        });
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        let actual = exec.eval("${{ runs_on.host }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("localhost"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.port }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("3000"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.user }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("some_user"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.userauth.type }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("password"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.userauth.password }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("some_password"))),
            Ok(ExprValue::Boolean(true))
        ));
    }

    #[test]
    pub fn runs_on_ssh_with_user_agent_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.runs_on = RunsOn::Ssh(SshConfig {
            host: "localhost".to_string(),
            port: "3000".to_string(),
            user: "some_user".to_string(),
            userauth: SshUserAuth::Agent,
        });
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        let actual = exec.eval("${{ runs_on.host }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("localhost"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.port }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("3000"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.user }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("some_user"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ runs_on.userauth.type }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("agent"))),
            Ok(ExprValue::Boolean(true))
        ));
    }

    #[test]
    pub fn runs_on_ssh_config_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.runs_on = RunsOn::SshFromGlobalConfig {
            ssh_config: "some_global_ssh_config".to_string(),
        };
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        let actual = exec.eval("${{ runs_on.ssh_config }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("some_global_ssh_config"))),
            Ok(ExprValue::Boolean(true))
        ));
    }
}
