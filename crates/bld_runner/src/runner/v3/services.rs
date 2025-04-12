use std::sync::Arc;

use anyhow::Result;
use bld_config::{BldConfig, SshUserAuth};
use bld_core::{context::Context, fs::FileSystem, logger::Logger, platform::{builder::{PlatformBuilder, PlatformOptions}, Image, Platform, SshAuthOptions, SshConnectOptions}, regex::RegexCache};

use crate::{expr::v3::context::CommonReadonlyRuntimeExprContext, pipeline::v3::Pipeline, registry::v3::Registry, runs_on::v3::RunsOn};

pub struct RunServices {
    pub config: Arc<BldConfig>,
    pub fs: Arc<FileSystem>,
    pub run_ctx: Arc<Context>,
    pub regex_cache: Arc<RegexCache>,
    pub expr_rctx: CommonReadonlyRuntimeExprContext,
    pub pipeline: Pipeline,
    pub platform: Arc<Platform>,
}

impl RunServices {
    pub async fn create(
        config: Arc<BldConfig>,
        fs: Arc<FileSystem>,
        run_ctx: Arc<Context>,
        regex_cache: Arc<RegexCache>,
        expr_rctx: CommonReadonlyRuntimeExprContext,
        pipeline: Pipeline,
        logger: Arc<Logger>,
    ) -> Result<Self> {
        let options = match &pipeline.runs_on {
            RunsOn::ContainerOrMachine(image) if image == "machine" => PlatformOptions::Machine,

            RunsOn::ContainerOrMachine(image) => PlatformOptions::Container {
                image: Image::Use(image),
                docker_url: None,
            },

            RunsOn::Pull {
                image,
                pull,
                docker_url,
                registry,
            } => {
                let image = if pull.unwrap_or_default() {
                    match registry.as_ref() {
                        Some(Registry::FromConfig(value)) => {
                            Image::pull(image, config.registry(value))
                        }
                        Some(Registry::Full(config)) => Image::pull(image, Some(config)),
                        None => Image::pull(image, None),
                    }
                } else {
                    Image::Use(image)
                };
                PlatformOptions::Container {
                    docker_url: docker_url.as_deref(),
                    image,
                }
            }

            RunsOn::Build {
                name,
                tag,
                dockerfile,
                docker_url,
            } => PlatformOptions::Container {
                image: Image::build(name, dockerfile, tag),
                docker_url: docker_url.as_deref(),
            },

            RunsOn::SshFromGlobalConfig { ssh_config } => {
                let config = config.ssh(ssh_config)?;
                let port = config.port.parse::<u16>()?;
                let auth = match &config.userauth {
                    SshUserAuth::Agent => SshAuthOptions::Agent,
                    SshUserAuth::Password { password } => SshAuthOptions::Password { password },
                    SshUserAuth::Keys {
                        public_key,
                        private_key,
                    } => SshAuthOptions::Keys {
                        public_key: public_key.as_deref(),
                        private_key,
                    },
                };
                PlatformOptions::Ssh(SshConnectOptions::new(
                    &config.host,
                    port,
                    &config.user,
                    auth,
                ))
            }

            RunsOn::Ssh(config) => {
                let port = config.port.parse::<u16>()?;
                let auth = match &config.userauth {
                    SshUserAuth::Agent => SshAuthOptions::Agent,
                    SshUserAuth::Password { password } => SshAuthOptions::Password { password },
                    SshUserAuth::Keys {
                        public_key,
                        private_key,
                    } => SshAuthOptions::Keys {
                        public_key: public_key.as_deref(),
                        private_key,
                    },
                };
                PlatformOptions::Ssh(SshConnectOptions::new(
                    &config.host,
                    port,
                    &config.user,
                    auth,
                ))
            }
        };

        let conn = run_ctx.get_conn();
        let platform = PlatformBuilder::default()
            .run_id(&expr_rctx.run_id)
            .config(config.clone())
            .options(options)
            .pipeline_env(&pipeline.env)
            .env(expr_rctx.env.clone())
            .logger(logger.clone())
            .conn(conn)
            .build()
            .await?;

        run_ctx.add_platform(platform.clone()).await?;
        Ok(Self {
            config,
            fs,
            run_ctx,
            regex_cache,
            expr_rctx,
            pipeline,
            platform,
        })
    }
}
