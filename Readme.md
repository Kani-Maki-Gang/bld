<p align="center">
    <img style="text-align: center;" height="200" width="200" src="./assets/logo.png">
</p>

# What is Bld
Bld is a CI/CD tool that targets to build pipelines both in a local environment and in a server.

# Building
Bld is built using the Rust programming language so you will need a [Rust installation](https://www.rust-lang.org/tools/install) in order to compile it.

### External dependencies
Additionally the project requires some external dependencies:
- build-essential
- pkg-config
- libssl-dev
- libsqlite3-dev
- (optional) docker
- (optional) trunk (for the server UI)
- (optional) nodejs (for the server UI)

> The package names are for Debian based distributions, install the appropriate packages on your distribution of choice.

### Build instructions
Once you have installed all of the above, you can build the project as follows
```bash
$ git clone https://github.com/Kani-Maki-Gang/bld.git
$ cd bld
$ cargo build --release
$ ./target/release/bld --version
```

> IMPORTANT! The bld_server crate requires for a `static_files` directory to exist in its project structure and if it doesn't build error will appear since it tries to embed all of its files to the resulting binary. There is a `build.rs` file for the project that creates the directory but if you encounter any issues, create the directory manually.

Bld also has a UI for its server that you can build it by running the below command
```bash
$ cd crates/bld_ui
$ trunk build
```

> Remember to have the trunk and nodejs installed in order to build the UI.

### Musl builds
Since there are multiple dependencies deployment of bld can be difficult, so the project supports targeting musl for static linking. If you have an existing bld binary locally built/installed then follow the below instructions. This steps require a docker installation.

```bash
$ bld run -p build-musl.yaml
$ ls dist
```

Or you can use cargo to built bld for your system and then run the pipelines for the musl build
```bash
$ cargo run -- run -p build-musl.yaml
$ ls dist
```

With the above a new container will be built with all the necessary dependencies for building the project and the bld pipeline will clone the repository, build the binary and the copy it in the musl/dist directory.

If a bld binary is not available, you will have to start a container with the bld-musl-builder and do the steps manually.

> The project currently targets only Linux. It has not been tested on Windows or Macos.

# Installation
For a prebuilt version of bld go to the github [releases](https://github.com/Kani-Maki-Gang/bld/releases) page and download the latest version.

# Creating a project
If you have followed the above Building section and have a Bld binary available, you can execute the below commands to initialize a Bld directory.
```bash
$ bld init
```
This will create a `.bld` directory with 2 files
- `config.yaml` Which contains the configuration for the current directory
- `default.yaml` The default pipeline which simply echos a 'hello world'

To test out the default pipeline simply execute
```bash
$ bld run -p default.yaml
```
The pipeline will execute on your machine, but it can be changed to target a docker container. Below is the same default pipeline but having the `runs-on` section changed to target an `ubuntu` docker image, just make sure it is installed.

```yaml
runs_on: ubuntu
version: 2
jobs:
  main:
  - echo 'hello world'
```

# Creating pipelines
In order to make new pipelines, you just need to create a yaml file under the `.bld` directory. For better structure you can add pipelines in directories and give the relative path to bld.

For example given a sample project you might want to have a build and deploy pipelines, the structure could be done as
```
.bld
 |    config.yaml
 |    default.yaml
 └─── sample
      |    build.yaml
      |    deploy.yaml
```
And the pipelines can be run with their relative path inside the `.bld` directory.
```bash
$ bld run -p sample/build.yaml
$ bld run -p sample/deploy.yaml
```

# Quick pipeline example
If you want a quick example of how a more involed pipeline would look, lets take the below example that tries to build a .net project and also run static analysis that will be sent to a sonar qube instance.

This is the example pipeline that runs the scanner called `example-project/scanner.yaml`
```yaml
name: Example .net project sonar scanner pipeline
version: 2
runs_on:
  dockerfile: /path/to/custom-dockerfile-for-scanner
  tag: latest
  name: scanner

variables:
  branch: master
  key: ExampleProject
  url: http://some-url-for-sonar-qube
  login: some_login_token

jobs:
  main:
  - git clone ${{branch}} https://some-url-for-the-repository
  - working_dir: /example-project/src
    exec:
    - dotnet sonarscanner begin /k:"${{key}}" /d:sonar.host.url=${{url}} /d:sonar.login="${{login}}"
    - dotnet build
    - dotnet sonarscanner end /d:sonar.login="${{login}}"
```

This is the example pipeline that builds the release version of the project called `example-project/build.yaml`
```yaml
name: Example project build pipeline
version: 2
runs_on:
  image: mcr.microsoft.com/dotnet/sdk:6.0-focal
  pull: true

variables:
  branch: master
  config: release

artifacts:
- method: get
  from: /example-project/src/ExampleProject/bin/${{config}}/net6.0/linux-x64
  to: /some/local/path/example-project/${{bld_run_id}}
  after: main

jobs:
  main:
  - git clone -b ${{branch}} https://some-url-for-the-repository
  - cd /example-project/src/ExampleProject && dotnet build -c ${{config}}
```

This is the example pipeline called `example-project/deploy.yaml` that runs on the host machine that initiates both pipelines in parallel and also makes a simple deployment of the release build.
```yaml
name: Example project deployment pipeline
version: 2
runs_on: machine

variables:
  branch: master

external:
- pipeline: example-project/sonar.yaml
  variables:
    branch: ${{branch}}

- pipeline: example-project/build.yaml
  variables:
    branch: ${{branch}}

jobs:
  scanner:
  - ext: example-project/scanner.yaml

  build_and_deploy:
  - ext: example-project/build.yaml
  - scp -r /some/local/path/example-project/${{bld_run_id}} user@some-ip:/some/path/to/the/server
```

> In the above the scanner pipeline runs parallel to the build and deploy since they are set in 2 different jobs. If everything should be run sequentially then the call to the scanner pipeline could be added to the same job as the other steps.

# Graceful shutdown
Since each run could create and run container as well as issue remote runs to bld servers, the cli handles the SIGINT and SIGTERM signals in order to properly cleanup all of the external components. To be noted that the stop command which stops a pipeline running on a server, can be used for a graceful early shutdown of a pipeline.

# Gihub action
Bld has an alpha version of a github action called [bld-action](https://github.com/marketplace/actions/bld-action) and you can access the repository [here](https://github.com/Kani-Maki-Gang/bld-github-action).

The action will look inside a repository to find the `.bld` directory in order to run the bld binary. An example that runs a pipeline with one variable is shown below:
```yaml
name: Demo workflow

on:
  push:
    branches: ["main"]

jobs:
  build_musl:
    runs-on: ubuntu-latest
    name: Run demo
    steps:
      - name: Checkout
        uses: actions/checkout@v3
      - name: Run pipeline
        uses: Kani-Maki-Gang/bld-github-action@v0.2.1-alpha
        with:
          pipeline: 'demo_pipeline.yaml'
          variables: |-
            branch=main
```

# The bld book
A more indepth look on bld's features can be found in the [bld book](https://kani-maki-gang.github.io/bld-book/) where you can find more topics such as:
- [Configuration](https://kani-maki-gang.github.io/bld-book/configuration.html)
- [Pipeline syntax](https://kani-maki-gang.github.io/bld-book/pipelines/preface.html)
- [How to run a server](https://kani-maki-gang.github.io/bld-book/configuration/server/running_a_server.html)
- [Cli information](https://kani-maki-gang.github.io/bld-book/cli/preface.html)
- [Examples](https://kani-maki-gang.github.io/bld-book/examples/preface.html)
