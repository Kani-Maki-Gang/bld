use crate::registry::v3::Registry;
use bld_config::SshConfig;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[cfg(feature = "all")]
use {
    crate::{
        expr::v3::{
            parser::Rule,
            traits::{EvalObject, ExprValue},
        },
        token_context::v3::{ApplyContext, ExecutionContext},
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

impl Display for RunsOn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContainerOrMachine(image) if image == "machine" => write!(f, "machine"),
            Self::ContainerOrMachine(image) => write!(f, "{image}"),
            Self::Pull { image, .. } => write!(f, "{image}"),
            Self::Build { name, tag, .. } => write!(f, "{name}:{tag}"),
            Self::SshFromGlobalConfig { ssh_config } => write!(f, "{}", ssh_config),
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
impl ApplyContext for RunsOn {
    async fn apply_context<C: ExecutionContext>(&mut self, ctx: &C) -> Result<()> {
        match self {
            RunsOn::Pull {
                image,
                registry,
                docker_url,
                ..
            } => {
                *image = ctx.transform(image.to_owned()).await?;
                if let Some(docker_url) = docker_url {
                    *docker_url = ctx.transform(docker_url.to_owned()).await?;
                }
                if let Some(registry) = registry.as_mut() {
                    registry.apply_context(ctx).await?;
                }
            }

            RunsOn::Build {
                name,
                tag,
                dockerfile,
                docker_url,
            } => {
                *name = ctx.transform(name.to_owned()).await?;
                *tag = ctx.transform(tag.to_owned()).await?;
                *dockerfile = ctx.transform(dockerfile.to_owned()).await?;
                *docker_url = if let Some(url) = docker_url {
                    Some(ctx.transform(url.to_owned()).await?)
                } else {
                    None
                };
            }

            RunsOn::ContainerOrMachine(image) if image != "machine" => {
                *image = ctx.transform(image.to_owned()).await?;
            }

            RunsOn::ContainerOrMachine(_) => {}

            RunsOn::Ssh(config) => {
                config.host = ctx.transform(config.host.to_owned()).await?;
                config.port = ctx.transform(config.port.to_owned()).await?;
                config.user = ctx.transform(config.user.to_owned()).await?;
                config.userauth = match &config.userauth {
                    SshUserAuth::Agent => SshUserAuth::Agent,
                    SshUserAuth::Keys {
                        public_key,
                        private_key,
                    } => {
                        let public_key = if let Some(pubkey) = public_key {
                            Some(ctx.transform(pubkey.to_owned()).await?)
                        } else {
                            None
                        };
                        let private_key = ctx.transform(private_key.to_owned()).await?;
                        SshUserAuth::Keys {
                            public_key,
                            private_key,
                        }
                    }
                    SshUserAuth::Password { password } => {
                        let password = ctx.transform(password.to_owned()).await?;
                        SshUserAuth::Password { password }
                    }
                }
            }

            RunsOn::SshFromGlobalConfig { ssh_config } => {
                *ssh_config = ctx.transform(ssh_config.to_owned()).await?;
            }
        }
        Ok(())
    }
}

#[cfg(feature = "all")]
impl<'a> EvalObject<'a> for RunsOn {
    fn eval_object(&'a self, _path: &mut Pairs<'_, Rule>) -> Result<ExprValue<'a>> {
        unimplemented!()
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
