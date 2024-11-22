use crate::{registry::v3::Registry, traits::{Validate, Validator}, validator::v3::{ErrorBuilder, KeywordValidator, SymbolValidator, Validatable}};
use bld_config::SshConfig;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[cfg(feature = "all")]
use anyhow::Result;

#[cfg(feature = "all")]
use bld_config::SshUserAuth;

#[cfg(feature = "all")]
use crate::token_context::v3::ExecutionContext;

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

    #[cfg(feature = "all")]
    pub async fn apply_tokens<'a>(&mut self, context: &ExecutionContext<'a>) -> Result<()> {
        match self {
            RunsOn::Pull {
                image,
                registry,
                docker_url,
                ..
            } => {
                *image = context.transform(image.to_owned()).await?;
                if let Some(docker_url) = docker_url {
                    *docker_url = context.transform(docker_url.to_owned()).await?;
                }
                if let Some(registry) = registry.as_mut() {
                    registry.apply_tokens(context).await?;
                }
            }

            RunsOn::Build {
                name,
                tag,
                dockerfile,
                docker_url,
            } => {
                *name = context.transform(name.to_owned()).await?;
                *tag = context.transform(tag.to_owned()).await?;
                *dockerfile = context.transform(dockerfile.to_owned()).await?;
                *docker_url = if let Some(url) = docker_url {
                    Some(context.transform(url.to_owned()).await?)
                } else {
                    None
                };
            }

            RunsOn::ContainerOrMachine(image) if image != "machine" => {
                *image = context.transform(image.to_owned()).await?;
            }

            RunsOn::ContainerOrMachine(_) => {}

            RunsOn::Ssh(ref mut config) => {
                config.host = context.transform(config.host.to_owned()).await?;
                config.port = context.transform(config.port.to_owned()).await?;
                config.user = context.transform(config.user.to_owned()).await?;
                config.userauth = match &config.userauth {
                    SshUserAuth::Agent => SshUserAuth::Agent,
                    SshUserAuth::Keys {
                        public_key,
                        private_key,
                    } => {
                        let public_key = if let Some(pubkey) = public_key {
                            Some(context.transform(pubkey.to_owned()).await?)
                        } else {
                            None
                        };
                        let private_key = context.transform(private_key.to_owned()).await?;
                        SshUserAuth::Keys {
                            public_key,
                            private_key,
                        }
                    }
                    SshUserAuth::Password { password } => {
                        let password = context.transform(password.to_owned()).await?;
                        SshUserAuth::Password { password }
                    }
                }
            }

            RunsOn::SshFromGlobalConfig { ssh_config } => {
                *ssh_config = context.transform(ssh_config.to_owned()).await?;
            }
        }
        Ok(())
    }
}

impl<'a> Validatable<'a> for RunsOn {
    fn validate<C>(&'a self, ctx: &mut C)
    where
        C: ErrorBuilder + SymbolValidator<'a> + KeywordValidator<'a>,
    {
        match &self {
            RunsOn::Build {
                name,
                tag,
                dockerfile,
                docker_url,
            } => {
                ctx.validate_symbols("runs_on > name", name);
                ctx.validate_symbols("runs_on > tag", tag);
                ctx.validate_symbols("runs_on > dockerfile", dockerfile);
                ctx.validate_file_path("runs_on > dockerfile", dockerfile);
                if let Some(docker_url) = docker_url {
                    ctx.validate_symbols("runs_on > docker_url", docker_url);
                    self.validate_docker_url(docker_url);
                }
            }

            RunsOn::Pull {
                image,
                docker_url,
                pull: _pull,
                registry,
            } => {
                self.validate_symbols("runs_on > image", image);
                if let Some(docker_url) = docker_url {
                    self.validate_symbols("runs_on > docker_url", docker_url);
                    self.validate_docker_url(docker_url);
                }
                if let Some(registry) = registry {
                    self.validate_registry("runs_on > registry", registry);
                }
            }

            RunsOn::ContainerOrMachine(value) => self.validate_symbols("runs_on", value),

            RunsOn::SshFromGlobalConfig { ssh_config } => {
                self.validate_symbols("runs_on > ssh_config", ssh_config);
                self.validate_global_ssh_config("runs_on > ssh_config", ssh_config);
            }

            RunsOn::Ssh(config) => {
                self.validate_symbols("runs_on > host", &config.host);
                self.validate_symbols("runs_on > port", &config.port);
                self.validate_symbols("runs_on > user", &config.user);
                match &config.userauth {
                    SshUserAuth::Agent => {}
                    SshUserAuth::Keys {
                        public_key,
                        private_key,
                    } => {
                        if let Some(pubkey) = public_key {
                            self.validate_symbols("runs_on > auth > public_key", pubkey);
                            self.validate_file_path("runs_on > auth > public_key", pubkey);
                        }
                        self.validate_symbols("runs_on > auth > private_key", private_key);
                        self.validate_file_path("runs_on > auth > private_key", private_key);
                    }
                    SshUserAuth::Password { password } => {
                        self.validate_symbols("runs_on > auth > password", password);
                    }
                }
            }
        }
    }
}
