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
name: Hello world pipeline
runs-on: machine
steps:
- name: Say hello
  exec:
  - echo hello world
```

This is a basic hello-world.container.yaml pipeline
```yaml
name: Hello world from a container
runs-on: ubuntu
steps:
- name: Say hello
  exec:
  - echo hello world
```

Run these 2 pipeline with
```bash
$ bld run -p hello-world.yaml
$ bld run -p hello-world.container.yaml
```

# Wiki
If you want to learn more about Bld, you can start by visiting some of the wiki pages available.
- [Getting started](https://github.com/kostas-vl/bld/wiki/Getting-started)
- [Building](https://github.com/kostas-vl/bld/wiki/Getting-started#building)
- [Pipeline syntax](https://github.com/kostas-vl/bld/wiki/Pipeline-syntax)
- [Server mode](https://github.com/kostas-vl/bld/wiki/Server-mode)
- [Configuration](https://github.com/kostas-vl/bld/wiki/Configuration)
