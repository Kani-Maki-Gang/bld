pub const VERSION: &str = "0.1";
pub const TOOL_DIR: &str = ".bld";
pub const DB_NAME: &str = "bld-server.db";
pub const PUSH: &str = "push";
pub const GET: &str = "get";
pub const ENV_TOKEN: &str = "bld:env:";
pub const VAR_TOKEN: &str = "bld:var:";
pub const RUN_PROPS_ID: &str = "bld:run:id";
pub const RUN_PROPS_START_TIME: &str = "bld:run:start-time";

pub const TOOL_DEFAULT_PIPELINE: &str = "default";
pub const TOOL_DEFAULT_PIPELINE_FILE: &str = "default.yaml";
pub const TOOL_DEFAULT_CONFIG: &str = "config";
pub const TOOL_DEFAULT_CONFIG_FILE: &str = "config.yaml";

pub const LOCAL_SERVER_HOST: &str = "127.0.0.1";
pub const LOCAL_SERVER_PORT: i64 = 6080;
pub const LOCAL_HA_MODE: bool = false;
pub const LOCAL_LOGS: &str = ".bld/logs";
pub const LOCAL_DB: &str = ".bld/db";
pub const LOCAL_SERVER_PIPELINES: &str = ".bld/server_pipelines";
pub const LOCAL_DOCKER_URL: &str = "tcp://127.0.0.1:2376";
pub const LOCAL_MACHINE_TMP_DIR: &str = ".bld/tmp";
pub const LOCAL_UNIX_SOCKET: &str = ".bld/server.sock";
pub const REMOTE_SERVER_NAME: &str = "demo_server";
pub const REMOTE_SERVER_HOST: &str = "127.0.0.1";
pub const REMOTE_SERVER_PORT: i64 = 6080;
pub const REMOTE_SERVER_OAUTH2: &str = ".bld/oauth2";

pub const DEFAULT_PIPELINE_CONTENT: &str = r"name: Default Pipeline
runs-on: machine
steps: 
- name: echo 
  exec:
  - echo 'hello world'
";

pub fn default_server_config() -> String {
    format!(
        r"local:
    ha-mode: {LOCAL_HA_MODE}
    host: {LOCAL_SERVER_HOST}
    port: {LOCAL_SERVER_PORT}
    logs: {LOCAL_LOGS}
    db: {LOCAL_DB}
    server-pipelines: {LOCAL_SERVER_PIPELINES}
    docker-url: {LOCAL_DOCKER_URL}"
    )
}

pub fn default_client_config() -> String {
    format!(
        r"local:
    docker-url: {LOCAL_DOCKER_URL}
remote:
    - server: {REMOTE_SERVER_NAME}
      host: {REMOTE_SERVER_HOST}
      port: {REMOTE_SERVER_PORT}"
    )
}
