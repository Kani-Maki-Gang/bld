pub const VERSION: &str = "0.2";
pub const TOOL_DIR: &str = ".bld";
pub const DB_NAME: &str = "bld-server.db";
pub const PUSH: &str = "push";
pub const GET: &str = "get";

pub const KEYWORD_BLD_DIR_V1: &str = "bld:root_dir";
pub const KEYWORD_ENV_V1: &str = "bld:env:";
pub const KEYWORD_VAR_V1: &str = "bld:var:";
pub const KEYWORD_RUN_PROPS_ID_V1: &str = "bld:run:id";
pub const KEYWORD_RUN_PROPS_START_TIME_V1: &str = "bld:run:start_time";

pub const KEYWORD_BLD_DIR_V2: &str = "bld_root_dir";
pub const KEYWORD_RUN_PROPS_ID_V2: &str = "bld_run_id";
pub const KEYWORD_RUN_PROPS_START_TIME_V2: &str = "bld_start_time";

pub const TOOL_DEFAULT_PIPELINE: &str = "default";
pub const TOOL_DEFAULT_PIPELINE_FILE: &str = "default.yaml";
pub const TOOL_DEFAULT_CONFIG: &str = "config";
pub const TOOL_DEFAULT_CONFIG_FILE: &str = "config.yaml";

pub const LOCAL_SERVER_HOST: &str = "127.0.0.1";
pub const LOCAL_SERVER_PORT: i64 = 6080;
pub const LOCAL_SERVER_PIPELINES: &str = ".bld/server_pipelines";
pub const LOCAL_SUPERVISOR_HOST: &str = "127.0.0.1";
pub const LOCAL_SUPERVISOR_PORT: i64 = 7080;
pub const LOCAL_SUPERVISOR_WORKERS: i64 = 5;
pub const LOCAL_HA_MODE: bool = false;
pub const LOCAL_LOGS: &str = ".bld/logs";
pub const LOCAL_DB: &str = ".bld/db";
pub const LOCAL_DOCKER_URL: &str = "tcp://127.0.0.1:2376";
pub const LOCAL_MACHINE_TMP_DIR: &str = ".bld/tmp";

pub const REMOTE_SERVER_NAME: &str = "demo_server";
pub const REMOTE_SERVER_HOST: &str = "127.0.0.1";
pub const REMOTE_SERVER_PORT: i64 = 6080;
pub const REMOTE_SERVER_AUTH: &str = ".bld/auth";

pub const DEFAULT_EDITOR: &str = "vi";

pub const DEFAULT_V1_PIPELINE_CONTENT: &str = r"runs_on: machine
version: 1
steps:
- exec:
  - echo 'hello world'
";

pub const DEFAULT_V2_PIPELINE_CONTENT: &str = r"runs_on: machine
version: 2
jobs:
  main:
  - echo 'hello world'
";

pub fn default_server_config() -> String {
    format!(
        r"local:
    ha-mode: {LOCAL_HA_MODE}
    server:
        host: {LOCAL_SERVER_HOST}
        port: {LOCAL_SERVER_PORT}
        pipelines: {LOCAL_SERVER_PIPELINES}
    supervisor:
        host: {LOCAL_SUPERVISOR_HOST}
        port: {LOCAL_SUPERVISOR_PORT}
        workers: {LOCAL_SUPERVISOR_WORKERS}
    logs: {LOCAL_LOGS}
    db: {LOCAL_DB}
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
