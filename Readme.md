# BLD
A simple CI/CD tool.

# Usage
```bash
# Command to create the .bld directory and a default pipeline.
bld init 

# Command to run the default pipeline.
bld run

# Command to run a specific pipeline on the local machine.
# pipeline_name should be a yaml file in the .bld directory.
bld run -p pipeline_name 

# Command to create the .bld directory for a bld server.
bld init -s

# Command to start bld in server mode.
bld server

# Command to push a local pipeline file to a server.
bld push -p pipeline_name -s server_name

# Command to run a pipeline on a server.
bld run -p pipeline_name -s server_name
```

# Commands
Command | Description
------- | -----------
config  | Lists bld's configuration.
init    | Initializes the bld configuration.
hist    | Fetches execution history of pipelines on a bld server.
ls      | Lists pipelines in a bld server.
monit   | Connects to a bld server to monitor the execution of a pipeline.
push    | Pushes the content of a pipeline to a bld server.
run     | Execute a bld pipeline.
server  | Start bld in server mode, listening to incoming build requests.
stop    | Stops a running pipeline on a server.


# Pipeline examples
#### Default pipeline
```yaml
name: Default Pipeline
runs-on: machine
steps: 
- name: echo 
  exec:
  - sh: echo 'hello world'
```

#### Build a dotnet core project
```yaml
name: dotnet core project ipeline
runs-on: mcr.microsoft.com/dotnet/core/sdk:3.1
steps:
- name: fetch repository
  exec:
  - sh: git clone https://github.com/project/project.git  
- name: build project
  working-dir: project
  exec:  
  - sh: dotnet build -c release
  - sh: cp -r bin/release/netcoreapp3.1/linux-x64/* /output
```

#### Build a node project
```yaml
name: node project pipeline
runs-on: node:12.18.3
steps:
- name: Fetch repository
  exec:
  - sh: git clone https://github.com/project/project.git
- name: install dependencies 
  working-dir: project
  exec:
  - sh: npm install
- name: build project 
  working-dir: project 
  exec:
  - sh: npm build 
```

#### Pipeline that invokes other pipelines
```yaml
name: pipeline that calls other pipelines
steps:
- name: Execute dotnet core pipeline
  call: dotnet_core_pipeline
- name: Execute nodejs pipeline 
  call: nodejs_pipeline
```

# What to do next
- [X] Automatic download of image if missing
- [X] Server mode.
- [X] Logging.
- [ ] Authentication. 
- [ ] Support for referencing ssh keys.
