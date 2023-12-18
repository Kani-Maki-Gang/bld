use anyhow::Result;
use bld_config::BldConfig;
use bollard::{Docker, API_DEFAULT_VERSION};

fn uses_http(url: &str) -> bool {
    url.starts_with("http:/") || url.starts_with("tcp:/")
}

fn sanitize_socket_path(url: &str) -> &str {
    url.strip_prefix("unix:/").unwrap_or(url)
}

pub fn docker(config: &BldConfig) -> Result<Docker> {
    let url = &config.local.docker_url;

    let docker = if uses_http(url) {
        Docker::connect_with_http(url, 120, API_DEFAULT_VERSION)?
    } else {
        let url = sanitize_socket_path(url);
        Docker::connect_with_socket(url, 120, API_DEFAULT_VERSION)?
    };

    Ok(docker)
}
