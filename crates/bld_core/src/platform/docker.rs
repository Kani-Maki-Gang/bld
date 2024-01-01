use anyhow::{anyhow, bail, Result};
use bld_config::{BldConfig, DockerUrl};
use bollard::{Docker, API_DEFAULT_VERSION};

fn uses_http(url: &str) -> bool {
    url.starts_with("http:/") || url.starts_with("tcp:/")
}

fn sanitize_socket_path(url: &str) -> &str {
    url.strip_prefix("unix:/").unwrap_or(url)
}

pub async fn docker(config: &BldConfig) -> Result<Docker> {
    match &config.local.docker_url {
        DockerUrl::SingleUrl(url) if uses_http(url) => {
            Docker::connect_with_http(url, 120, API_DEFAULT_VERSION).map_err(|e| anyhow!(e))
        }

        DockerUrl::SingleUrl(url) => {
            let url = sanitize_socket_path(url);
            Docker::connect_with_socket(url, 120, API_DEFAULT_VERSION).map_err(|e| anyhow!(e))
        }

        DockerUrl::MultipleUrls(urls) => {
            let instances: Vec<Docker> = urls
                .iter()
                .flat_map(|(_, url)| {
                    if uses_http(url) {
                        Docker::connect_with_http(url, 120, API_DEFAULT_VERSION)
                    } else {
                        let url = sanitize_socket_path(url);
                        Docker::connect_with_socket(url, 120, API_DEFAULT_VERSION)
                    }
                })
                .collect();

            for instance in instances.into_iter() {
                if instance.ping().await.is_ok() {
                    return Ok(instance);
                }
            }

            bail!("unable to connect to any of the available docker endpoints");
        }
    }
}
