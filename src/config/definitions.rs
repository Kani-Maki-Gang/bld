pub const VERSION: &str = "0.1";
pub const TOOL_DIR: &str = ".bld";
pub const DB_NAME: &str = "bld-server.db";
pub const PUSH: &str = "push";
pub const GET: &str = "get";
pub const VAR_TOKEN: &str = "bld:var:";

pub const TOOL_DEFAULT_PIPELINE: &str = "default";
pub const TOOL_DEFAULT_PIPELINE_FILE: &str = "default.yaml";
pub const TOOL_DEFAULT_CONFIG: &str = "config";
pub const TOOL_DEFAULT_CONFIG_FILE: &str = "config.yaml";

pub const LOCAL_SERVER_HOST: &str = "127.0.0.1";
pub const LOCAL_SERVER_PORT: i64 = 6080;
pub const LOCAL_HA_MODE: bool = false;
pub const LOCAL_LOGS: &str = ".bld/logs";
pub const LOCAL_DB: &str = ".bld/db";
pub const LOCAL_DOCKER_URL: &str = "tcp://127.0.0.1:2376";
pub const LOCAL_MACHINE_TMP_DIR: &str = ".bld/tmp";
pub const REMOTE_SERVER_NAME: &str = "demo_server";
pub const REMOTE_SERVER_HOST: &str = "127.0.0.1";
pub const REMOTE_SERVER_PORT: i64 = 6080;
pub const REMOTE_SERVER_OAUTH2: &str = ".bld/oauth2";

pub const DEFAULT_PIPELINE_CONTENT: &str = r"name: Default Pipeline
runs-on: machine
steps: 
- name: echo 
  exec:
  - sh: echo 'hello world'
";

pub fn default_server_config() -> String {
    format!(
        r"local:
    ha-mode: {}
    host: {}
    port: {}
    logs: {}
    db: {}
    docker-url: {}",
        LOCAL_HA_MODE, LOCAL_SERVER_HOST, LOCAL_SERVER_PORT, LOCAL_LOGS, LOCAL_DB, LOCAL_DOCKER_URL
    )
}

pub fn default_client_config() -> String {
    format!(
        r"local:
    docker-url: {}
remote:
    - server: {}
      host: {}
      port: {}",
        LOCAL_DOCKER_URL, REMOTE_SERVER_NAME, REMOTE_SERVER_HOST, REMOTE_SERVER_PORT
    )
}
