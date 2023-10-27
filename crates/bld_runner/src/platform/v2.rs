use std::fmt::Display;

use anyhow::Result;
use bld_config::{LibvirtConfig, SshConfig, SshUserAuth};
use serde::{Deserialize, Serialize};

use crate::token_context::v2::PipelineContext;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Platform {
    ContainerOrMachine(String),
    Pull {
        image: String,
        pull: bool,
    },
    Build {
        name: String,
        tag: String,
        dockerfile: String,
    },
    Libvirt(LibvirtConfig),
    LibvirtFromGlobalConfig {
        libvirt_config: String,
    },
    Ssh(SshConfig),
    SshFromGlobalConfig {
        ssh_server: String,
    },
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContainerOrMachine(image) if image == "machine" => write!(f, "machine"),
            Self::ContainerOrMachine(image) => write!(f, "{image}"),
            Self::Pull { image, .. } => write!(f, "{image}"),
            Self::Build { name, tag, .. } => write!(f, "{name}:{tag}"),
            Self::Libvirt(config) => write!(f, "{} - {}", config.uri, config.domain),
            Self::LibvirtFromGlobalConfig { libvirt_config } => write!(f, "{libvirt_config}"),
            Self::SshFromGlobalConfig { ssh_server } => write!(f, "{}", ssh_server),
            Self::Ssh(config) => write!(f, "{}:{}", config.host, config.port),
        }
    }
}

impl Platform {
    pub fn default_ssh_port() -> String {
        String::from("22")
    }

    pub async fn apply_tokens<'a>(&mut self, context: &PipelineContext<'a>) -> Result<()> {
        match self {
            Platform::Pull { image, .. } => {
                *image = context.transform(image.to_owned()).await?;
            }

            Platform::Build {
                name,
                tag,
                dockerfile,
            } => {
                *name = context.transform(name.to_owned()).await?;
                *tag = context.transform(tag.to_owned()).await?;
                *dockerfile = context.transform(dockerfile.to_owned()).await?;
            }

            Platform::ContainerOrMachine(image) if image != "machine" => {
                *image = context.transform(image.to_owned()).await?;
            }

            Platform::ContainerOrMachine(_) => {}

            Platform::Libvirt(ref mut config) => {
                config.uri = context.transform(config.uri.to_owned()).await?;
                config.domain = context.transform(config.domain.to_owned()).await?;
                config.start_before_run = match &config.start_before_run {
                    Some(value) => Some(context.transform(value.to_owned()).await?),
                    None => None,
                };
                config.shutdown_after_run = match &config.shutdown_after_run {
                    Some(value) => Some(context.transform(value.to_owned()).await?),
                    None => None,
                };
            }

            Platform::LibvirtFromGlobalConfig { libvirt_config } => {
                *libvirt_config = context.transform(libvirt_config.to_owned()).await?;
            }

            Platform::Ssh(ref mut config) => {
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

            Platform::SshFromGlobalConfig { ssh_server } => {
                *ssh_server = context.transform(ssh_server.to_owned()).await?;
            }
        }
        Ok(())
    }
}
