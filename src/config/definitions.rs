pub static VERSION: &str = "0.1";
pub static TOOL_DIR: &str = ".bld";
pub static DB_NAME: &str = "bld-server.db";

pub static TOOL_DEFAULT_PIPELINE: &str = "default";
pub static TOOL_DEFAULT_PIPELINE_FILE: &str = "default.yaml";
pub static TOOL_DEFAULT_CONFIG: &str = "config";
pub static TOOL_DEFAULT_CONFIG_FILE: &str = "config.yaml";

pub static LOCAL_SERVER_MODE: bool = false;
pub static LOCAL_SERVER_HOST: &str = "127.0.0.1";
pub static LOCAL_SERVER_PORT: i64 = 6080;
pub static LOCAL_LOGS: &str = ".bld/logs";
pub static LOCAL_DB: &str = ".bld/db";
pub static LOCAL_DOCKER_URL: &str = "tcp://127.0.0.1:2376";
pub static REMOTE_SERVER_NAME: &str = "demo_server";
pub static REMOTE_SERVER_HOST: &str = "127.0.0.1";
pub static REMOTE_SERVER_PORT: i64 = 6080;
pub static REMOTE_SERVER_OAUTH2: &str = ".bld/oauth2";

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
    server-mode: {} 
    host: {}
    port: {}
    logs: {}
    db: {}
    docker-url: {}",
        true, LOCAL_SERVER_HOST, LOCAL_SERVER_PORT, LOCAL_LOGS, LOCAL_DB, LOCAL_DOCKER_URL
    )
}

pub fn default_client_config() -> String {
    format!(
        r"local:
    server-mode: {} 
    docker-url: {}
remote:
    - server: {}
      host: {}
      port: {}",
        LOCAL_SERVER_MODE,
        LOCAL_DOCKER_URL,
        REMOTE_SERVER_NAME,
        REMOTE_SERVER_HOST,
        REMOTE_SERVER_PORT
    )
}
