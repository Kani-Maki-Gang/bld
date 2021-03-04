# BLD
A simple CI/CD tool.

# Features
- [X] Running a pipeline on the executing machine or on a docker container.
- [X] Client and Server mode. 
- [X] Copying artifacts to and from a container.
- [X] Authentication using an oauth2 service (Github, Google, Microsoft etc).

# Commands
Command | Description
------- | -----------
config  | Lists bld's configuration.
init    | Initializes the bld configuration.
hist    | Fetches execution history of pipelines on a bld server.
login   | Initiates the login process for a bld server
ls      | Lists pipelines in a bld server.
monit   | Connects to a bld server to monitor the execution of a pipeline.
push    | Pushes the content of a pipeline to a bld server.
run     | Execute a bld pipeline.
server  | Start bld in server mode, listening to incoming build requests.
stop    | Stops a running pipeline on a server.

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
artifacts:
- method: push
  from: /some/path
  to: /some/path/in/the/container
  ignore-errors: false
  after-steps: false
- method: get
  from: /some/path/in/the/container
  to: /some/path
  ignore-errors: false
  after-steps: true
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
artifacts:
- method: push
  from: /some/path
  to: /some/path/in/the/container
  ignore-errors: false
  after-steps: false
- method: get
  from: /some/path/in/the/container
  to: /some/path
  ignore-errors: false
  after-steps: true
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

# Authentication

Server mode does not have it's own authentication method but it uses external authentication services. In the future multiple ways of
authentication will be supported. The only current method is using an existing oauth2 service (Github, Google, Microsoft etc). 
Below is an example of authentication using a Github oauth2 app.

#### Configuration of client to login using github
The below example assumes that a github oauth2 app has been setup.
```yaml
local:
  docker-url: tcp://127.0.0.1:2376
remote:
  - server: local_srv 
    host: 127.0.0.1
    port: 6080
    auth:
      method: oauth2 
      auth-url: https://github.com/login/oauth/authorize
      token-url: https://github.com/login/oauth/access_token
      client-id: your_oauth2_app_client_id 
      client-secret: your_oauth2_app_client_secret 
      scopes: ["public_repo", "user:email"]
  - server: local_srv_2
    host: 127.0.0.1
    port: 6090
    same-auth-as: local_srv
```

#### Configuration of server to validate user using github
This will send a request to the provided validation url in order to fetch the user info.
```yaml
local:
    enable-server: true 
    auth:
      method: oauth2
      validation-url: https://api.github.com/user
    host: 127.0.0.1
    port: 6080
    logs: .bld/logs
    db: .bld/db
    docker-url: tcp://127.0.0.1:2376
```

#### Login process
```bash
# Use the login command to generate a url that will provide you with a code and state 
# tokens used for the auth process
bld login

# Or use -s to specify the server name
bld login -s local_srv

Open the printed url in a browser in order to login with the specified oauth2 provider.

https://github.com/login/oauth/authorize?response_type=code&client_id=your_oauth2_client_id&state=some_state_token&code_challenge=some_generated_code_challenge&code_challenge_method=the_code_challenge_method&redirect_uri=http%3A%2F%2F127.0.0.1%3A6080%2FauthRedirect&scope=public_repo+user%3Aemail

After logging in input both the provided code and state here.
code:

state:

# At this point by navigating to the generated url you will be able to get the code and state. Copy it to your terminal and a new
# token will be created under .bld/oauth2 directory on a file with the target server as name.
```

# What to do next
- [ ] High availability mode.
