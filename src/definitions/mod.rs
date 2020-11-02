pub static VERSION: &str = "0.1";
pub static TOOL_DIR: &str = ".bld";

pub static TOOL_DEFAULT_PIPELINE: &str = "default";
pub static TOOL_DEFAULT_CONFIG: &str = "config";

pub static LOCAL_ENABLE_SERVER: bool = false;
pub static LOCAL_SERVER_HOST: &str = "127.0.0.1";
pub static LOCAL_SERVER_PORT: i64 = 6080;
pub static LOCAL_LOGS: &str = ".bld/logs";
pub static LOCAL_DB: &str = ".bld/db";
pub static LOCAL_DOCKER_HOST: &str = "127.0.0.1";
pub static LOCAL_DOCKER_INSECURE_PORT: i64 = 2375;
pub static LOCAL_DOCKER_SECURE_PORT: i64 = 2376;
pub static LOCAL_DOCKER_USE_TLS: bool = false;
pub static REMOTE_SERVER_NAME: &str = "demo_server";
pub static REMOTE_SERVER_HOST: &str = "127.0.0.1";
pub static REMOTE_SERVER_PORT: i64 = 6080;

pub static DEFAULT_PIPELINE_CONTENT: &str = r"name: Default Pipeline
runs-on: machine
steps: 
- name: echo 
  exec:
  - sh: echo 'hello world'
";

pub fn default_server_config() -> String {
    format!(
        r"local:
    enable-server: {} 
    host: {}
    port: {}
    logs: {}
    db: {}
    docker-host: {}
    docker-port: {}
    docker-use-tls: {}",
        true,
        LOCAL_SERVER_HOST,
        LOCAL_SERVER_PORT,
        LOCAL_LOGS,
        LOCAL_DB,
        LOCAL_DOCKER_HOST,
        LOCAL_DOCKER_INSECURE_PORT,
        LOCAL_DOCKER_USE_TLS
    )
}

pub fn default_client_config() -> String {
    format!(
        r"local:
    enable-server: {} 
    docker-host: {}
    docker-port: {}
    docker-use-tls: {}
remote:
    - server: {}
      host: {}
      port: {}",
        LOCAL_ENABLE_SERVER,
        LOCAL_DOCKER_HOST,
        LOCAL_DOCKER_INSECURE_PORT,
        LOCAL_DOCKER_USE_TLS,
        REMOTE_SERVER_NAME,
        REMOTE_SERVER_HOST,
        REMOTE_SERVER_PORT
    )
}
