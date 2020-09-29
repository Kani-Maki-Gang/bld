# BLD
A simple CI/CD tool build on rust

# Usage
```bash
# Command to create the .bld directory and a default pipeline.
bld init 

# Command to run the default pipeline.
bld run

# Command to run a specific pipeline.
# pipeline_name should be a yaml file in the .bld directory.
bld run -p pipeline_name 
```

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

# What to do next
- [X] Automatic download of image if missing
- [ ] Server mode.
- [ ] Logging.
- [ ] Parallel run of pipelines
- [ ] Support for referencing ssh keys.
- [ ] Distributed mode
