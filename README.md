# AME

AME(Artificial MLOPS Engineer) automates model training and orchestration of related tasks. 

Long term the goal is to service the entire lifecycle of machine learning models but for now the focus is on training and orchestration. AME is designed to abstract away the details regarding infrastructure, so data scientists can focus on what is important. While at the same time AME enables a high degree of configurability.

**Note that AME is still in early development, see this [issue](https://github.com/TeaInSpace/ame/issues/69) for the current status.**

**A few highlights**:
- Simple declarative machine learning pipelines, through a minimal yaml file.
- No special integrations are required in your python code, it should just work.
- Easy handling of data sources and destinations.
- Intelligent scaling of cloud resources, within the limits you set.
- Highly portable, anywhere you can spin up a k8s cluster AME will run.
- Jupylab support, use AME's scaling to spin up jupyter lab instances when needed.
- Git tracking, AME will track any repository or organisation you grant access too and automatically detect when an AME file is created or updated.

## Quick start

### Installation:

TODO: Fill this out when the CLI is available.

### Connect the CLI to the server

Before the CLI can execute Tasks it needs to be connected with an AME server. Use the setup command to get started.

![setup](https://user-images.githubusercontent.com/10332534/194759967-4b0d80b8-eab7-4350-9d23-5b02b51440d3.gif)

### Training a model

#### Initialize a project

All AME Tasks are executed within the context a project. To setup a project in a directory, run `ame create project`. This will generate
an AME file(ame.yaml) containing the name of a project. When you create Tasks, they will be placed here.

#### Create a Task

To create a Task run `ame create task`. AME supports single step tasks and multi step Task pipelines, the CLI will guide you through creating either one. You will be asked to supply the necessary information for AME to execute the Task successfully, such as compute cpu, gpu, env variables etc.

#### Run it

Once a task is created, it can be run. Use `ame run` to select a task and run it. The logs will be shown in the terminal as if you were running the task on your local machine. Any Artifacts generated will be transferred back to your local directory.

This gif demonstrates these steps, note that it is sped up to keep the length to a minimum:
![readme](https://user-images.githubusercontent.com/10332534/196032105-869531c3-ebea-44cf-9cee-e57f0546dcda.gif)


### Scheduled training

Ame supports scheduling Tasks to run on a recurring basis using a cron schedule, as long as there is a git repository where ame can clone the project from. Run `ame schedule task` to schedule a Task.

![image](https://user-images.githubusercontent.com/10332534/196032747-f5e65c1a-a183-491a-9512-7762df070349.png)

### Exploring

The CLI can be used to explore and change the state of your AME instance. This includes viewing logs for running tasks, 

### Handling secrets and environment variables


## Core concepts

AME has a few concepts which users should be familiar with.

Note that the examples below all show yaml files to demonstrate what the various configurations look like but one of the goals of AME is to minimize the time spent on manually writing and debugging yaml configuration. Therefore when using AME you will not normally have to manually write, edit or debug yaml files.

### Tasks

Ame uses Tasks as building blocks, a Task defines a piece of work to be done. This includes what command to run, which compute resources are required and various other configuration options. Tasks are defined in the ame project file and are expected to live in the git repository in a similar manner to Github actions, Gitlab CI etc.

```yaml
#ame.yaml
projectname: bestproject
tasks:
  download_data:
    runcommand: python download_data.py
    secrets:
      name: storagesecret
      envkey: STORAGE_SECRET
    env:
      envkey: PROJECT_ENVIRONMENT
      value: production
    resources:
      cpu: 1
      memory: 2Gi
      storage: 50Gi
```

### Projects
A project consists of a directory of files containing the AME file and must have a unique name. It provides the context to which any Task is executed. This allows AME to run your code as if it were running locally on your machine. For example if you have an experiment which produces artifacts and you run that experiment using AME, all of the artifacts will appear locally in your directory as if you had run the experiment on your local machine.

TODO: show an example of this.

### Pipelines

You might have multiple Tasks meant to be executed together, for example downloading data, preparing data, model training, model upload. Each of these Tasks will have different requirements. This can be expressed using a pipeline. Each Task in a pipeline is executed in a separate container potentially on different machines if their compute requirements are different. To ensure that your code will work without modification, all of the state is transferred between steps transparently so it appears as if all of the steps are executed on the same machine. For example data is downloaed in step 1, prepared in step 2 and trained on in step 3 AME will make sure to transfer these files automatically between steps so no adjustments are reqired to the project's code.

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

RecurringTasks, concists of a Task, a cron schedule and a reference to a git repository. Currently the only way to schedule recurring tasks is through the CLI.

## Deployment and administration

AME is Kubernetes native, it will play nicely with any existing Kubernetes setup you may have and is very gitops friendly.

TODO: Fill in details

## Architecture

AME is designed to be run within a Kubernetes cluster and therefore consists of multiple custom resource definitions, controllers a gRPC+REST server and a CLI.
Eventually a graphical interface will be developed aswell.

![Untitled Diagram drawio](https://user-images.githubusercontent.com/10332534/195980196-06fbf347-19a2-48eb-915d-44008bd606e7.png)

### Dependencies

AME relies on Argo Workflows as a workflow engine and minio for object storage at the moment.

TODO: Fill architecture details

## Roadmap

TODO
