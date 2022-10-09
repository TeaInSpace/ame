# AME

AME(Artificial MLOPS Engineer) automates model training and orchestration of related tasks. 

Long term the goal is to service the entire lifecycle of machine learning models but for now we focus on training and orchestration. AME is designed to make many decisions so users & admins don't have to, while still keeping them informed and in control.

**Note that AME is still in early development, see the list further down the readme of which features are implemented.**

**A few highlights**:
- Simple declarative machine learning pipelines, through a minimal yaml file.
- No special integrations are required in your python code, it should just work.
- Easy handling of data sources and destinations.
- Intelligent scaling of cloud resources, within the limits you set.
- Highly portable, anywhere you can spin up a k8s cluster AME will run.
- Jupylab support, use AME's scaling to spin up jupyter lab instances when needed.
- Git tracking, AME will track any repository or organisation you grant access too and automatically detect when an AME file is created or updated.

An example of running an adoc task with the CLI:
![out](https://user-images.githubusercontent.com/10332534/192134166-0eddbeae-853e-40f7-bd56-cb3400899c0f.gif)

## Feature overview

### MVP:
The initial MVP of AME will be able to do the following, implemented features have a checkmark:

- [x] Run your model training with zero changes to your python code.
- [x] Execute tasks remotely, but display output and downloads artifacts to simulate local execution.
- [ ] Automatic setup of python versions and dependencies using any common python dependency manager.
- [x] Scale the infrastructure used for the model training in a code effective manner.
- [x] Provide simple authentication when using the CLI.
- [x] Support injecting secrets during Task execution.
- [x] Support configuration environment variables for Task execution.
- [ ] Adjust the resource requirements to fit the needs of your training
automatically, for example if you training fails due to lack of ram, the task
will be rescheduled and more ram provisioned.
- [x] Schedule reoccuring model traning based on project specifications.
- [x] Configure how your project is run declaritively in a config file.
- [ ] Support outputting artifacts from your training to an arbitrary location
such as an s3 bucket.
- [x] Support storing artifacts within AME's own object storage.
- [ ] Deployable with a terraform setup supplied from the project repository.
- [ ] AME will be able to track projects in your git repositories and detect
when they have an AME config file. Allowing you to simply place your config file
in the repo like you would with CI/CD configuration and let AME take care of the
rest.
- [ ] Display project runs in a graphical UI, perhaps using mlflow.
- [ ] Support using Jupyterlab with AME.
- [ ] Support using custom images.
- [ ] Support overriding default workflow behavior.


### Core concepts

AME has a few simple concepts which users should be familiar with.

#### Tasks

Ame uses tasks as building blocks, a task defines a piece of work to be done. This includes what command to run, which compute resources are required and various other configuration options. Tasks are defined in the ame project file, it will live in your git repository in a similar manner to Github actions, Gitlab CI etc.

```yaml
#ame.yaml
projectname: bestproject
tasks:
  download_data:
    runcommand: python downloaddata.py
    secrets:
      name: storagesecret
      envkey: STORAGE_SECRET
    env:
      envkey: PROJECT_ENVIRONMENT
      value: production
    resources:
      cpu: 2
      memory: 8Gi
      storage: 50Gi 
```

#### Projects
A project is the directory of files that you are working from. It provides the context to which any Task is executed. This allows AME to run your code as if it were running locally on your machine. For example if you have an experiment which produces artifacts and you run that experiment using AME, all of the artifacts will appear locally in your directory as if you had run the experiment on your local machine.

TODO: show an example of this.

### Pipelines

You might have multiple Tasks meant to be executed together, for example downloading data, preparing data, model training, model upload. Each of these Tasks will have different requirements. This can be expressed using a pipeline.
```yaml
#ame.yaml
projectname: bestproject
tasks:
  main:
    pipeline:
      download_data:
        runcommand: python downloaddata.py
        secrets:
          name: storagesecret
          envkey: STORAGE_SECRET
        env:
          envkey: PROJECT_ENVIRONMENT
          value: production
        resources:
          cpu: 2
          memory: 8Gi
          storage: 50Gi
      prepare_data:
        runcommand: python preparadata.py
        resources:
          cpu: 8
          memory: 16Gi
          storage: 50Gi
       train_model:
        runcommand: python train.py
        resources:
          cpu: 4
          memory: 8Gi
          storage: 30Gi
          gpus: 1
          vram: 24Gi
       upload_model:
         runcommand: python train.py
         resources:
          cpu: 2
          memory: 4Gi
          storage: 30Gi
```
### Recurring tasks

AME currently only supports time based scheduling, for example at 12pm every day. You can do that by adding a recurring Task to the AME file in your repository. Recurring tasks only require the schedule and a reference to a Task it should execute.
```yaml
#ame.yaml
projectname: bestproject
recurring_tasks:
  daily_training:
    taskref: main
    schedule: 0 12 * * *
tasks:
  main:
    pipeline:
      download_data:
        runcommand: python downloaddata.py
        secrets:
          name: storagesecret
...

```

## Getting started with the CLI

### Connect the CLI to the server

Before the CLI can execute Tasks it needs to be connected with an AME server. Use the setup command to get started.

![setup](https://user-images.githubusercontent.com/10332534/194759967-4b0d80b8-eab7-4350-9d23-5b02b51440d3.gif)

### Initialize a project

All AME Tasks are executed with the context a project. To setup a project, run `ame create project`.

### Creating a Task

To create a Task use the `ame create task` command. AME supports single step tasks and multi stap Task pipeliens, the CLI will guide you through creating either one. You will be asked to supply the necessary information for AME to executr the Task successfully.

### Executing tasks from the CLI

Once a task is created, it can be run. Use `ame run` to select a task and run it. The logs will be shown in the terminal as if you were running the task on your local machine. Any Artifacts generated will be transferred back to your local directory.

### Scheduling tasks for recurring execution

Ame supports Tasks that are run on repeatedly using a cron tab, as long as there is a git repository where ame can download the project from.

When you are first starting out you will probably be experimenting with AME. The AME CLI supports running tasks remotely using what ever project you are working within at the time. In other words you can execute your python code on a remote machine, but still see output from the execution and get any artifacts locally, essentially mirroring the expericence of running your code locally, except you can request what ever compute resources you may need. 

TODO: insert gif of executing a task.

### Configuring your project from the CLI

In order to guarantee a valid configuratinon and avoid wasting time on finding YAML errors, the CLI can build the config for your and ensure that it is valid.

TODO: insert gif of building yaml config.

### Override defaults

TODO

#### Custom images

TODO



## Deployment and administration

AME is Kubernetive native, it will play nicely with any existing Kubenertes setup you may. If you do not have an existing see our examples in this repo for how to spin up a production ready Cluster with AME in your environment of choice.

TODO

## Architecture

AME is designed to be run within a Kubernetes cluster and therefores consists of multiple custom resource definitions, controllers a gRPC+REST server and a CLI.
Eventually a a graphical interface will be developed aswell.

TODO

## Roadmap

TODO
