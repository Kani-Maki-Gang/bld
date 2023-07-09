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

> The package names are for Debian based distributions, install the appropriate packages on your distribution of choice.

### Build instructions
Once you have installed all of the above, you can build the project as follows
```bash
$ git clone https://github.com/Kani-Maki-Gang/bld.git
$ cd bld
$ cargo build --release
$ ./target/release/bld --version
```

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

# The bld book
A more indepth look on bld's features can be found in the [bld book](https://kani-maki-gang.github.io/bld-book/) where you can find more topics such as:
- [Pipeline syntax](https://kani-maki-gang.github.io/bld-book/pipelines/version2.html)
- [Configuration](https://kani-maki-gang.github.io/bld-book/configuration/preface.html)
- [How to run a server](https://kani-maki-gang.github.io/bld-book/configuration/server/running_a_server.html)
- [Cli information](https://kani-maki-gang.github.io/bld-book/cli/preface.html)
- [Examples](https://kani-maki-gang.github.io/bld-book/examples/preface.html)
