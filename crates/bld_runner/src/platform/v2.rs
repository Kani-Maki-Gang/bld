use std::fmt::Display;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::token_context::v2::PipelineContext;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PlatformSshAuth {
    Keys {
        public_key: Option<String>,
        private_key: String,
    },
    Password {
        password: String,
    },
    Agent,
}

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
    Libvirt {
        vm: String,
        start_before_run: String,
        shutdown_after_run: String,
    },
    Ssh {
        host: String,
        #[serde(default = "Platform::default_ssh_port")]
        port: String,
        user: String,
        userauth: PlatformSshAuth,
    },
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContainerOrMachine(image) if image == "machine" => write!(f, "machine"),
            Self::ContainerOrMachine(image) => write!(f, "{image}"),
            Self::Pull { image, .. } => write!(f, "{image}"),
            Self::Build { name, tag, .. } => write!(f, "{name}:{tag}"),
            Self::Libvirt { vm, .. } => write!(f, "{vm}"),
            Self::Ssh { host, port, .. } => write!(f, "{host}:{port}"),
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
            Platform::Libvirt {
                vm,
                start_before_run,
                shutdown_after_run,
            } => {
                *vm = context.transform(vm.to_owned()).await?;
                *start_before_run = context.transform(start_before_run.to_owned()).await?;
                *shutdown_after_run = context.transform(shutdown_after_run.to_owned()).await?;
            }
            Platform::Ssh {
                host,
                port,
                user,
                userauth: auth,
            } => {
                *host = context.transform(host.to_owned()).await?;
                *port = context.transform(port.to_owned()).await?;
                *user = context.transform(user.to_owned()).await?;
                match auth {
                    PlatformSshAuth::Agent => {}
                    PlatformSshAuth::Keys {
                        public_key,
                        private_key,
                    } => {
                        if let Some(pubkey) = public_key {
                            *public_key = Some(context.transform(pubkey.to_owned()).await?);
                        }
                        *private_key = context.transform(private_key.to_owned()).await?;
                    }
                    PlatformSshAuth::Password { password } => {
                        *password = context.transform(password.to_owned()).await?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
