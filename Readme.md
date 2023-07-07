<p align="center">
    <img style="text-align: center;" height="200" width="200" src="./assets/logo.png">
</p>

# BLD
A simple and BLAZINGLY fast CI/CD tool.

# Features
- [x] Running a pipeline locally on the executing machine or on a docker container.
- [x] Parent and child execution of pipelines.
- [x] Client and Server mode.
- [ ] Servers in cluster mode (future).

# Hello world

This is a basic hello-world.yaml pipeline
```yaml
version: 1
runs_on: machine
steps:
- exec:
  - echo hello world
```

This is a basic hello-world.container.yaml pipeline
```yaml
version: 1
runs_on: ubuntu
steps:
- exec:
  - echo hello world
```

This is calling the hello-world.yaml pipeline from another pipeline
```yaml
version: 1
runs_on: ubuntu
steps:
- exec:
  - ext: hello-world.yaml
```

Run these 3 pipeline with
```bash
$ bld run -p hello-world.yaml
$ bld run -p hello-world.container.yaml
$ bld run -p parent.yaml
```

# Wiki
If you want to learn more about Bld, you can start by visiting some of the wiki pages available.
- [Getting started](https://github.com/kostas-vl/bld/wiki/Getting-started)
- [Building](https://github.com/kostas-vl/bld/wiki/Getting-started#building)
- [Pipeline syntax](https://github.com/kostas-vl/bld/wiki/Pipeline-syntax)
- [Server mode](https://github.com/kostas-vl/bld/wiki/Server-mode)
- [Configuration](https://github.com/kostas-vl/bld/wiki/Configuration)
