# BLD
A simple CI/CD tool build on rust

# Usage
Command to create the .bld directory and a default pipeline.
```bash
bld init 
```

Command to run the default pipeline.
```bash
bld run
```

Command to run a specific pipeline.
```bash
bld run -p pipeline_name // pipeline_name should be .yaml file in the .bld directory.
```

# Pipeline example
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
name: BurstChat API Pipeline
runs-on: mcr.microsoft.com/dotnet/core/sdk:3.1
steps:- name: Fetch repository
  exec:
  - sh: git clone https://github.com/super-cool-project.git  
- name: Build project
  exec:  
  - sh: dotnet build super-cool-project/SuperCoolProject.csproj -c release
  - sh: cp -r burstchat/src/BurstChat.Api/bin/release/netcoreapp3.1/linux-x64/* output
```

# What to do next
1. Automatic download of image if missing
2. Parallel runs of pipelines
3. Daemon and logging
4. Support for referencing ssh keys.
5. Distributed mode