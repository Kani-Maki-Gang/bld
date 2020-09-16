pub static VERSION: &str = "0.1";
pub static AUTHOR: &str = "Kostas Vl";
pub static TOOL_DIR: &str = ".bld";
pub static TOOL_DEFAULT_PIPELINE: &str = "default";
pub static DEFAULT_PIPELINE_CONTENT: &str = r"name: Default Pipeline
runs-on: machine
steps: 
- name: echo 
  exec:
  - sh: echo 'hello world'
";
