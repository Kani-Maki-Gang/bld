use anyhow::{anyhow, bail, Result};
use bld_config::{BldConfig, DockerUrl, DockerUrlEntry};
use bollard::{Docker, API_DEFAULT_VERSION};

fn uses_http(url: &str) -> bool {
    url.starts_with("http:/") || url.starts_with("tcp:/")
}

fn sanitize_socket_path(url: &str) -> &str {
    url.strip_prefix("unix:/").unwrap_or(url)
}

pub fn docker(config: &BldConfig, name: Option<&str>) -> Result<Docker> {
    match &config.local.docker_url {
        DockerUrl::Single(url) if uses_http(url) => {
            Docker::connect_with_http(url, 120, API_DEFAULT_VERSION).map_err(|e| anyhow!(e))
        }

        DockerUrl::Single(url) => {
            let url = sanitize_socket_path(url);
            Docker::connect_with_socket(url, 120, API_DEFAULT_VERSION).map_err(|e| anyhow!(e))
        }

        DockerUrl::Multiple(urls) if name.map(|x| urls.contains_key(x)).is_some() => {
            let (DockerUrlEntry::Url(url) | DockerUrlEntry::UrlWithDefault { url, .. }) = name
                .and_then(|x| urls.get(x))
                .ok_or_else(|| anyhow!("unable to find docker url entry in config"))?;

            if uses_http(url) {
                Docker::connect_with_http(url, 120, API_DEFAULT_VERSION).map_err(|e| anyhow!(e))
            } else {
                let url = sanitize_socket_path(url);
                Docker::connect_with_socket(url, 120, API_DEFAULT_VERSION).map_err(|e| anyhow!(e))
            }
        }

        DockerUrl::Multiple(urls) => {
            let instances: Vec<Docker> = urls
                .iter()
                .filter(|(_, x)| x.is_default())
                .flat_map(|(_, x)| x.get_url_with_default())
                .flat_map(|url| {
                    if uses_http(url) {
                        Docker::connect_with_http(url, 120, API_DEFAULT_VERSION)
                    } else {
                        let url = sanitize_socket_path(url);
                        Docker::connect_with_socket(url, 120, API_DEFAULT_VERSION)
                    }
                })
                .collect();

            if instances.len() > 1 {
                bail!("multiple default docker urls defined in config");
            }

            instances
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("no default docker url defined in config"))
        }
    }
}
