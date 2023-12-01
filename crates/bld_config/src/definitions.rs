pub const VERSION: &str = "0.3.1";
pub const TOOL_DIR: &str = ".bld";
pub const PUSH: &str = "push";
pub const GET: &str = "get";

pub const KEYWORD_BLD_DIR_V1: &str = "bld:root_dir";
pub const KEYWORD_ENV_V1: &str = "bld:env:";
pub const KEYWORD_VAR_V1: &str = "bld:var:";
pub const KEYWORD_RUN_PROPS_ID_V1: &str = "bld:run:id";
pub const KEYWORD_RUN_PROPS_START_TIME_V1: &str = "bld:run:start_time";

pub const KEYWORD_BLD_DIR_V2: &str = "bld_root_dir";
pub const KEYWORD_PROJECT_DIR_V2: &str = "bld_project_dir";
pub const KEYWORD_RUN_PROPS_ID_V2: &str = "bld_run_id";
pub const KEYWORD_RUN_PROPS_START_TIME_V2: &str = "bld_start_time";

pub const TOOL_DEFAULT_PIPELINE: &str = "default";
pub const TOOL_DEFAULT_PIPELINE_FILE: &str = "default.yaml";
pub const TOOL_DEFAULT_CONFIG: &str = "config";
pub const TOOL_DEFAULT_CONFIG_FILE: &str = "config.yaml";

pub const LOCAL_SERVER_HOST: &str = "127.0.0.1";
pub const LOCAL_SERVER_PORT: i64 = 6080;
pub const LOCAL_SERVER_PIPELINES: &str = "server_pipelines";
pub const LOCAL_SUPERVISOR_HOST: &str = "127.0.0.1";
pub const LOCAL_SUPERVISOR_PORT: i64 = 7080;
pub const LOCAL_SUPERVISOR_WORKERS: i64 = 5;
pub const LOCAL_HA_MODE: bool = false;
pub const LOCAL_LOGS: &str = "logs";
pub const LOCAL_DEFAULT_DB_DIR: &str = "db";
pub const LOCAL_DEFAULT_DB_NAME: &str = "bld-server.db";
pub const LOCAL_DOCKER_URL: &str = "tcp://127.0.0.1:2376";
pub const LOCAL_MACHINE_TMP_DIR: &str = "tmp";

pub const REMOTE_SERVER_NAME: &str = "demo_server";
pub const REMOTE_SERVER_HOST: &str = "127.0.0.1";
pub const REMOTE_SERVER_PORT: i64 = 6080;
pub const REMOTE_SERVER_AUTH: &str = "auth";

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
