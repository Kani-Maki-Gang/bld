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
        validator::v3::{ExprScope, Validate, ValidatorContext},
    },
    anyhow::Result,
    bld_config::{DockerUrl, RegistryConfig, SshUserAuth},
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
        #[serde(default)]
        volumes: Vec<String>,
    },
    Build {
        name: String,
        tag: String,
        dockerfile: String,
        docker_url: Option<String>,
        #[serde(default)]
        volumes: Vec<String>,
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

    pub fn volumes(&self) -> &[String] {
        match self {
            RunsOn::Pull { volumes, .. } | RunsOn::Build { volumes, .. } => volumes,
            _ => &[],
        }
    }

    /// Returns a copy of the current instance with every expression replaced by its value.
    #[cfg(feature = "all")]
    pub fn resolve<F: Fn(&str) -> Result<String>>(&self, eval: F) -> Result<Self> {
        let eval_opt = |value: &Option<String>| -> Result<Option<String>> {
            value.as_deref().map(&eval).transpose()
        };
        let eval_all =
            |values: &[String]| -> Result<Vec<String>> { values.iter().map(|x| eval(x)).collect() };

        let value = match self {
            Self::ContainerOrMachine(image) => Self::ContainerOrMachine(eval(image)?),

            Self::Pull {
                image,
                registry,
                pull,
                docker_url,
                volumes,
            } => Self::Pull {
                image: eval(image)?,
                registry: registry.as_ref().map(|x| x.resolve(&eval)).transpose()?,
                pull: *pull,
                docker_url: eval_opt(docker_url)?,
                volumes: eval_all(volumes)?,
            },

            Self::Build {
                name,
                tag,
                dockerfile,
                docker_url,
                volumes,
            } => Self::Build {
                name: eval(name)?,
                tag: eval(tag)?,
                dockerfile: eval(dockerfile)?,
                docker_url: eval_opt(docker_url)?,
                volumes: eval_all(volumes)?,
            },

            Self::Ssh(config) => Self::Ssh(SshConfig {
                host: eval(&config.host)?,
                port: eval(&config.port)?,
                user: eval(&config.user)?,
                userauth: match &config.userauth {
                    SshUserAuth::Agent => SshUserAuth::Agent,
                    SshUserAuth::Password { password } => SshUserAuth::Password {
                        password: eval(password)?,
                    },
                    SshUserAuth::Keys {
                        public_key,
                        private_key,
                    } => SshUserAuth::Keys {
                        public_key: eval_opt(public_key)?,
                        private_key: eval(private_key)?,
                    },
                },
            }),

            Self::SshFromGlobalConfig { ssh_config } => Self::SshFromGlobalConfig {
                ssh_config: eval(ssh_config)?,
            },
        };

        Ok(value)
    }
}

#[cfg(feature = "all")]
impl Registry {
    fn resolve<F: Fn(&str) -> Result<String>>(&self, eval: F) -> Result<Self> {
        let value = match self {
            Self::FromConfig(config) => Self::FromConfig(eval(config)?),

            Self::Full(config) => Self::Full(RegistryConfig {
                url: eval(&config.url)?,
                username: config.username.as_deref().map(&eval).transpose()?,
                password: config.password.as_deref().map(&eval).transpose()?,
            }),
        };

        Ok(value)
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
                ..
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
                ..
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
                volumes,
            } => {
                ctx.push_section("name");
                ctx.validate_expressions(name, ExprScope::StartOfRun);
                ctx.pop_section();

                ctx.push_section("tag");
                ctx.validate_expressions(tag, ExprScope::StartOfRun);
                ctx.pop_section();

                ctx.push_section("dockerfile");
                ctx.validate_expressions(dockerfile, ExprScope::StartOfRun);
                ctx.validate_file_path(dockerfile);
                ctx.pop_section();

                if let Some(docker_url) = docker_url {
                    validate_docker_url(ctx, docker_url);
                }

                validate_volumes(ctx, volumes);
            }

            RunsOn::Pull {
                image,
                docker_url,
                pull: _pull,
                registry,
                volumes,
            } => {
                ctx.push_section("image");
                ctx.validate_expressions(image, ExprScope::StartOfRun);
                ctx.pop_section();

                if let Some(docker_url) = docker_url {
                    validate_docker_url(ctx, docker_url);
                }
                if let Some(registry) = registry.as_ref() {
                    validate_registry(ctx, registry);
                }

                validate_volumes(ctx, volumes);
            }

            RunsOn::ContainerOrMachine(value) => {
                ctx.validate_expressions(value, ExprScope::StartOfRun)
            }

            RunsOn::SshFromGlobalConfig { ssh_config } => {
                validate_global_ssh_config(ctx, ssh_config);
            }

            RunsOn::Ssh(config) => {
                ctx.push_section("host");
                ctx.validate_expressions(&config.host, ExprScope::StartOfRun);
                ctx.pop_section();

                ctx.push_section("port");
                if ctx.contains_expressions(&config.port) {
                    ctx.validate_expressions(&config.port, ExprScope::StartOfRun);
                } else if config.port.parse::<u16>().is_err() {
                    ctx.append_error(&format!(
                        "'{}' is not a valid port number (must be 0-65535)",
                        config.port
                    ));
                }
                ctx.pop_section();

                ctx.push_section("user");
                ctx.validate_expressions(&config.user, ExprScope::StartOfRun);
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
                            ctx.validate_expressions(pubkey, ExprScope::StartOfRun);
                            ctx.validate_file_path(pubkey);
                            ctx.pop_section();
                        }

                        ctx.push_section("private_key");
                        ctx.validate_expressions(private_key, ExprScope::StartOfRun);
                        ctx.validate_file_path(private_key);
                        ctx.pop_section();
                    }

                    SshUserAuth::Password { password } => {
                        ctx.push_section("password");
                        ctx.validate_expressions(password, ExprScope::StartOfRun);
                        ctx.pop_section();
                    }
                }
                ctx.pop_section();
            }
        }
    }
}

#[cfg(feature = "all")]
fn validate_volumes<'a, C: ValidatorContext<'a>>(ctx: &mut C, volumes: &'a [String]) {
    ctx.push_section("volumes");

    for volume in volumes {
        ctx.validate_expressions(volume, ExprScope::StartOfRun);

        // A volume entry must be of the form `host_path:container_path` (optionally
        // followed by `:ro`/`:rw` etc.). We only check that a separator is present;
        // the concrete paths may still contain expressions resolved at runtime.
        if !ctx.contains_expressions(volume) && !volume.contains(':') {
            ctx.append_error(&format!(
                "'{volume}' is not a valid volume, expected 'host_path:container_path'"
            ));
        }
    }

    ctx.pop_section();
}

#[cfg(feature = "all")]
fn validate_docker_url<'a, C: ValidatorContext<'a>>(ctx: &mut C, value: &'a str) {
    ctx.push_section("docker_url");

    if ctx.contains_expressions(value) {
        ctx.validate_expressions(value, ExprScope::StartOfRun);
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
            ctx.validate_expressions(&config.url, ExprScope::StartOfRun);
            ctx.pop_section();

            if let Some(username) = &config.username {
                ctx.push_section("username");
                ctx.validate_expressions(username, ExprScope::StartOfRun);
                ctx.pop_section();
            }

            if let Some(password) = &config.password {
                ctx.push_section("password");
                ctx.validate_expressions(password, ExprScope::StartOfRun);
                ctx.pop_section();
            }
        }
    }

    ctx.pop_section();
}

#[cfg(feature = "all")]
fn validate_global_registry_config<'a, C: ValidatorContext<'a>>(ctx: &mut C, value: &'a str) {
    if ctx.contains_expressions(value) {
        ctx.validate_expressions(value, ExprScope::StartOfRun);
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

    if ctx.contains_expressions(value) {
        ctx.validate_expressions(value, ExprScope::StartOfRun);
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
    use bld_config::{BldConfig, SshConfig, SshUserAuth};
    use bld_core::fs::FileSystem;
    use bld_pkg::PackageManager;
    use bld_utils::sync::IntoArc;

    use crate::{
        expr::v3::{
            context::CommonReadonlyRuntimeExprContext,
            exec::CommonExprExecutor,
            traits::{EvalExpr, ExprText, ExprValue, MockWritableRuntimeExprContext},
        },
        job::v3::Job,
        pipeline::v3::Pipeline,
        registry::v3::Registry,
        step::v3::{ShellCommand, Step},
        validator::v3::{CommonValidator, ConsumeValidator, ValidatorWritableRuntimeExprContext},
    };

    use super::RunsOn;

    async fn validate_runs_on(runs_on: RunsOn) -> anyhow::Result<()> {
        let job_name = "main";
        let config = BldConfig::default().into_arc();
        let file_system = FileSystem::local(config.clone()).into_arc();
        let package_manager = PackageManager::new(config.clone()).into_arc();
        let expr_rctx = CommonReadonlyRuntimeExprContext::default();
        let expr_wctx = vec![ValidatorWritableRuntimeExprContext::new(job_name)];

        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            job_name.to_string(),
            Job {
                runs_on,
                steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                    id: "build".to_string(),
                    run: "echo hello".to_string(),
                    ..Default::default()
                }))],
                ..Default::default()
            },
        );

        CommonValidator::new(
            &pipeline,
            config,
            file_system,
            package_manager,
            &expr_rctx,
            &expr_wctx,
        )
        .unwrap()
        .validate()
        .await
    }

    #[tokio::test]
    pub async fn runs_on_with_runtime_expr_validation_failure() {
        let data = vec![
            RunsOn::ContainerOrMachine("${{ steps.build.outputs.image }}".to_string()),
            RunsOn::Pull {
                image: "${{ matrix.image }}".to_string(),
                registry: None,
                pull: Some(true),
                docker_url: None,
                volumes: vec![],
            },
            RunsOn::Pull {
                image: "ubuntu:22.04".to_string(),
                registry: None,
                pull: Some(true),
                docker_url: None,
                volumes: vec!["${{ matrix.dir }}:/work".to_string()],
            },
            RunsOn::Build {
                name: "${{ steps.build.outputs.name }}".to_string(),
                tag: "latest".to_string(),
                dockerfile: "Dockerfile".to_string(),
                docker_url: None,
                volumes: vec![],
            },
        ];

        for runs_on in data {
            let Err(e) = validate_runs_on(runs_on).await else {
                panic!("expected a validation error for a runtime expression");
            };
            assert!(
                e.to_string()
                    .contains("is not available at the start of a run"),
                "{e}"
            );
        }
    }

    #[test]
    pub fn runs_on_pull_deserializes_volumes() {
        let yaml = r#"
image: my-image:latest
pull: true
volumes:
  - /host/.claude:/home/rust/.claude
  - ${{ inputs.worktree_dir }}:${{ inputs.worktree_dir }}
"#;
        let runs_on: RunsOn = serde_yaml_ng::from_str(yaml).unwrap();
        match runs_on {
            RunsOn::Pull { image, volumes, .. } => {
                assert_eq!(image, "my-image:latest");
                assert_eq!(
                    volumes,
                    vec![
                        "/host/.claude:/home/rust/.claude".to_string(),
                        "${{ inputs.worktree_dir }}:${{ inputs.worktree_dir }}".to_string(),
                    ]
                );
            }
            other => panic!("expected a Pull runs_on, got {other:?}"),
        }
    }

    #[test]
    pub fn runs_on_pull_volumes_default_empty() {
        let runs_on: RunsOn = serde_yaml_ng::from_str("image: my-image:latest").unwrap();
        match runs_on {
            RunsOn::Pull { volumes, .. } => assert!(volumes.is_empty()),
            other => panic!("expected a Pull runs_on, got {other:?}"),
        }
    }

    #[test]
    pub fn runs_on_machine_expr_eval_success() {
        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            "main".to_string(),
            Job {
                runs_on: RunsOn::ContainerOrMachine("machine".to_string()),
                ..Default::default()
            },
        );
        wctx.expect_get_exec_id().returning(|| Some("main"));
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);
        let expected = ExprValue::Text(ExprText::Ref("machine"));
        let actual = exec.eval("${{ runs_on }}").unwrap();
        assert!(matches!(
            actual.try_eq(&expected),
            Ok(ExprValue::Boolean(true))
        ));
    }

    #[test]
    pub fn runs_on_container_name_expr_eval_success() {
        let mut wctx = MockWritableRuntimeExprContext::new();
        wctx.expect_get_exec_id().returning(|| Some("main"));

        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert("main".to_string(), Job::default());

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
            let Some(job) = pipeline.jobs.get_mut("main") else {
                panic!("no main job found");
            };
            job.runs_on = RunsOn::ContainerOrMachine(value.to_string());

            let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);
            let actual = exec.eval("${{ runs_on }}").unwrap();
            assert!(matches!(
                actual.try_eq(&expected),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }

    #[test]
    pub fn runs_on_pull_image_expr_eval_success() {
        let mut wctx = MockWritableRuntimeExprContext::new();
        wctx.expect_get_exec_id().returning(|| Some("main"));

        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            "main".to_string(),
            Job {
                runs_on: RunsOn::Pull {
                    image: "ubuntu:latest".to_string(),
                    registry: Some(Registry::FromConfig("registry-config".to_string())),
                    pull: Some(true),
                    docker_url: Some("docker-url".to_string()),
                    volumes: vec![],
                },
                ..Default::default()
            },
        );
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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
        let mut wctx = MockWritableRuntimeExprContext::new();
        wctx.expect_get_exec_id().returning(|| Some("main"));

        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            "main".to_string(),
            Job {
                runs_on: RunsOn::Build {
                    name: "test-image".to_string(),
                    tag: "1.3.4".to_string(),
                    dockerfile: "path-to-dockerfile".to_string(),
                    docker_url: Some("docker-url".to_string()),
                    volumes: vec![],
                },
                ..Default::default()
            },
        );
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let actual = exec.eval("${{ runs_on.name }}").unwrap();
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
        let mut wctx = MockWritableRuntimeExprContext::new();
        wctx.expect_get_exec_id().returning(|| Some("main"));
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            "main".to_string(),
            Job {
                runs_on: RunsOn::Ssh(SshConfig {
                    host: "localhost".to_string(),
                    port: "3000".to_string(),
                    user: "some_user".to_string(),
                    userauth: SshUserAuth::Keys {
                        public_key: Some("some_public_key".to_string()),
                        private_key: "some_private_key".to_string(),
                    },
                }),
                ..Default::default()
            },
        );
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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
        let mut wctx = MockWritableRuntimeExprContext::new();
        wctx.expect_get_exec_id().returning(|| Some("main"));

        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            "main".to_string(),
            Job {
                runs_on: RunsOn::Ssh(SshConfig {
                    host: "localhost".to_string(),
                    port: "3000".to_string(),
                    user: "some_user".to_string(),
                    userauth: SshUserAuth::Password {
                        password: "some_password".to_string(),
                    },
                }),
                ..Default::default()
            },
        );
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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
        let mut wctx = MockWritableRuntimeExprContext::new();
        wctx.expect_get_exec_id().returning(|| Some("main"));

        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            "main".to_string(),
            Job {
                runs_on: RunsOn::Ssh(SshConfig {
                    host: "localhost".to_string(),
                    port: "3000".to_string(),
                    user: "some_user".to_string(),
                    userauth: SshUserAuth::Agent,
                }),
                ..Default::default()
            },
        );
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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
        let mut wctx = MockWritableRuntimeExprContext::new();
        wctx.expect_get_exec_id().returning(|| Some("main"));

        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            "main".to_string(),
            Job {
                runs_on: RunsOn::SshFromGlobalConfig {
                    ssh_config: "some_global_ssh_config".to_string(),
                },
                ..Default::default()
            },
        );
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let actual = exec.eval("${{ runs_on.ssh_config }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("some_global_ssh_config"))),
            Ok(ExprValue::Boolean(true))
        ));
    }
}
