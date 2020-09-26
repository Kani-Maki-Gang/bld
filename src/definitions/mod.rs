pub static VERSION: &str = "0.1";
pub static TOOL_DIR: &str = ".bld";
pub static TOOL_DEFAULT_PIPELINE: &str = "default";
pub static TOOL_DEFAULT_CONFIG: &str = "config";

pub static DEFAULT_PIPELINE_CONTENT: &str = r"name: Default Pipeline
runs-on: machine
steps: 
- name: echo 
  exec:
  - sh: echo 'hello world'
";

pub static DEFAULT_CONFIG_CONTENT: &str = r"host: 127.0.0.1
port: 2375
";
