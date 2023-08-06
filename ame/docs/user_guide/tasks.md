# Tasks

`Tasks` are the basic building block for most of AME's functionality. A `Task` represents some work to be done, often in the form of python code to be executed
but in principle it can be anything executable in a container.

`Tasks` are configured in an AME file `ame.yml`. 

```yaml
# ame.yml

tasks:
  - name: train_my_model
    executor:
      !poetry
      pythonVersion: 3.11
      command: python train.py
```

This is an example of a the absolute minimal requirements for a task, a name and an executor. An executor specifies how a task should be executed, a complete list can found (here)[].
In this case AME will ensure that the specified python version is present and use poetry to install dependencies and enter a shell before executing the command `python train.py`. 

To run the task manually simply enter the directory with the file and project and run `ame task run -l train_my_model`. The `-l` ensures that we are using the local context and not trying
to run a remote task already present in the AME cluster. Alternatively if you don't want to type the name omit it and AME will present a list of the available `Tasks`.

This page provides an overview of working with Task's using the [cli](TODO) and [dashboard]() as well as reference of all configuration options. 

## Working with Tasks

### Cli commands

The AME [cli](TODO) contains a subcommand `ame task` which is for operating on Tasks.

#### View deployed Tasks

```shell
ame task list

Name                                                        Status    Project 
ameprojectsrc56k2rlogregtrainingsklearn-logistic-regression Succeeded Unknown 
ameprojectsrcj8264logregtrainingdatasetdemo                 Failed    Unknown 
datasetdatasetdemodataset1dataset1-fetcher                  Succeeded Unknown 
validate-logreg-1                                           Succeeded Unknown 

```

#### Run a task

Individual task's can be run with `ame task run`. This will look for Tasks in your current project. Upload any files necessary and execute the task. 

#### View task logs

Task logs for any running task can be viewed with `ame task logs`.


## Task Reference

Reference for all configuration for AME Tasks.
 
### Resource requirements

By default a task gets limited resources allocated TODO: what is the default? which can be fine for very simple tasks but anything non trivial will require. To change the default see
how to configure default (here)[].

Resources include any computational resources that needs to be allocated:

- CPU
- GPU
- memory
- storage

They can be specified for a task with the resources field:


```yaml
# ame.yml

tasks:
  - name: train_my_model
    executor:
      !poetry
      pythonVersion: 3.11
      command: python train.py
    resources:
      memory: 10G # 10 Gigabyte ram
      cpu: 4 # 4 CPU threads
      storage: 30G # 30Gigabyte of disk space
      nvidia.com/gpu: 1 # 1 Nvidia GPU
```

AME uses style of string units for resources as Kubernetes. If you are not familiar with that no worries!, the readon for the details.

**Memory and storage units**: memory units are measured in bytes. Integers with quantity suffixes can be used to express memory quantities. 
See the following table for complete list of available units. 

| Unit               	| Suffix 	| Example 	|
|--------------------	|--------	|---------	|
| Exabyte            	| E      	| 2.5E    	|
| Exbibyte           	| Ei     	| 2.5Ei   	|
| Terabyte           	| T      	| 2.5T    	|
| Tebibyte           	| Ti     	| 2.5Ti   	|
| Gigabyte           	| G      	| 2.5G    	|
| Gibibyte           	| Gi     	| 2.5Gi   	|
| Megabyte           	| M      	| 2.5M    	|
| Mebibyte           	| Mi     	| 2.5Mi   	|
| Kilobyte           	| k      	| 2.5k    	|
| Kibibyte           	| Ki     	| 2.5Ki   	|
| Byte               	|        	| 2.5     	|
| Fraction of a Byte 	| m      	| 2500m   	|

TODO: make better example
**Example**:
```
  128974848, 129e6, 129M,  128974848000m, 123Mi
```

**CPU units**

`1` specifies a single CPU thread either virtual or hyperthreaded depending on the underlying machine. `0.5` specifies half a CPU thread and so does `500m`.
`1=1000m`, `m` stands for millicpu/millicores.

**GPU units**

GPU scheduling in AME is current pretty barebones. You must specifie whole GPUs, meaning no fractional units and you can't ask for a specific model of device only vendor.
For cases where different GPU models need to be differentiate node labels can be used as a work around. This is essentially the way Kubernetes solves GPU scheduling as well, however we will
be abstracting all this away in the coming release of AME to allow for requesting specific models and fractional GPU sharing directly in the Task specification, see the tracking (issue)[].
For details on how to use node labels with GPUs see (this)[].

### Secrets

Task's will often require access to private resources such as objectstorage, databases or APIs. AME provides a built in secret store as well as integration with (Vault)[https://www.vaultproject.io/].
This section will walk through how to use secrets with a Task. For details on AME's secret store and how to integrate with a Vault instance see the relevant (documentation)[].

**Quick example**

```yaml
# ame.yml

tasks:
  - name: train_my_model
    executor:
      !poetry
      pythonVersion: 3.11
      command: python train.py
    resources:
      memory: 10G # 10 Gigabyte ram
      cpu: 4 # 4 CPU threads
      storage: 30G # 30Gigabyte of disk space
      nvidia.com/gpu: 1 # 1 Nvidia GPU
    secrets:
     - !ame # Secret stored by AME
       key: blob_storage_key # Key identifying a secret in AME's secret store.
       injectAs: MY_BLOB_STORAGE_SECRET # This will inject an environment variable with the name MY_BLOB_STORAGE_SECRET and with the value of the secret.

     # TODO this does not cover all vault cases
     - !vault # Secret stored in Vault
       vault: company_vault # Name of the vault to use.
       key: path/to/secret # Path to the secret in Vault.
       injectAs: COMPANY_SECRET  # This will inject an environment variable with the name COMPANY_SECRET and with the value of the secret.
```

**Explanation**

`!ame` and `!vault` indicate the type of secret being specified. `key` is the key that identifies the secret for both variants.
`injectAs` specifies the name of the environment variable. `vault` specifies the name of the vault to use.

A secret can be out in AME's secret store using the secret sub command: `ame secret add`. The will prompts will ask for a key and value.

### Container images

The default image is intended to cover most cases. It uses the latest LTS version of Ubuntu with a non slim environment. Most projects should just work inside
this environment. How ever there a few reasons you might want to replace the default image with your own. 

**Special library requiremetents**: if a package you are using requires some system library that is not installed by default you can address this by creating a custom image. In this 
case it probably makes sense to take the base AME image and extend it with the dependencies you need. If you think this dependency should be included in the base image feel free to create
and issue on Github.

**Security**: depending on your needs a full blown Ubuntu environment might have too many security issues due to all of the packages installed by default. In this case creating a minimal image with the
exact requirements you is the way to go. See guide on doing that [here]().

TODO: how do we ignore large files?

### Task cleanup

### Executors

An executor describes how AME should execute a task. It does this by providing enough information for AME to know how dependencies should be installed and how a Task is run. 

All executes support overriding the default image with the image field. Read more about the default image used by AME and overriding it with your own [here]().

TODO: How to change versions of dependency managers???

#### Quick examples

##### Poetry executor

```yaml
# ame.yml

tasks:
  - name: train_my_model
    executor:
      !poetry
      pythonVersion: 3.11
      command: python train.py
```

##### Pip executor

```yaml
# ame.yml

tasks:
  - name: train_my_model
    executor:
      !pip
      pythonVersion: 3.11
      command: python train.py
```

##### Pipenv executor

```yaml
# ame.yml

tasks:
  - name: train_my_model
    executor:
      !pipenv
      command: python train.py
```

##### MLflow executor

```yaml
# ame.yml

tasks:
  - name: train_my_model
    executor:
      !mlflow
      pythonVersion: 3.11
      command: python train.py # TODO should the command be specified here??
```

##### Custom executor


```yaml
# ame.yml

tasks:
  - name: train_my_model
    executor:
      !custom
      pythonVersion: 3.11
      command: python train.py # TODO should the command be specified here??
```

#### Poetry executor

The Poetry executor expects a Poetry compatible project. This means a pyproject.taml and poetry.lock file should be present. 
Note that the python version is required to be specified, in the future this information will be extracted from pyproject.toml.
The value  in the command field is executed using poetry run and it used to start the task.

```yaml
    executor:
      !poetry
      pythonVersion: 3.11
      command: python train.py
```

#### Pipenv executor

The Pipenv executor expects a Pipenv compatible project. This means a Pipfile and Pipfile.lock file should be present. 
The python version is installed by pipenv following the value in the Pipfile.
The value  in the command field is executed inside a shell created with pipenv shell.

```yaml
    executor:
      !pipenv
      command: python train.py
```

#### Pip executor

The expects a project where pip can install dependencies. This means a requirements.txt file should be present. We strongly recommend that
versions are specified in the requirements.txt file to ensure that the project will run just like on your local machine.
The value  in the command field is executed inside a virtual environment created used venv with the dependencies installed by pip.

```yaml
    executor:
      !pip
      pythonVersion: 3.11
      command: python train.py
```

#### Custom executor

The custom executor is meant for special cases where the other executors are insufficient. For example if you are not using python.
No setup is performed in this case, the command is simply executed inside a container.

```yaml
    executor:
      !pip
      pythonVersion: 3.11
      command: python train.py
      image: myimage
```
### Common task examples

### Templates

If you find yourself repeating a lot of Task configuration it might be useful to create templates for common configuration. Templates are partial Tasks that can be used
as the base for a Task, any fields set in the Task will then override the Template. The combination of Task and Template fields must yield a valid Task.

#### Quick examples

##### Template and Task in the same project
```yaml
# ame.yml

tasks:
  - name: train_my_model
    fromTemplate: 
      name: xgboost_resources
    executor:
      !poetry
      pythonVersion: 3.11
      command: python train.py

templates:
  - name: xgboost_resources # Note that this is the name of the template
    resources:
      memory: 10G 
      cpu: 4 
      storage: 30G 
      nvidia.com/gpu: 1 
    secrets:
     - !ame # Secret stored by AME
       key: blob_storage_key # Key identifying a secret in AME's secret store.
       injectAs: MY_BLOB_STORAGE_SECRET # This will inject an environment variable with the name MY_BLOB_STORAGE_SECRET and with the value of the secret.

     # TODO this does not cover all vault cases
     - !vault # Secret stored in Vault
       vault: company_vault # Name of the vault to use.
       key: path/to/secret # Path to the secret in Vault.
       injectAs: COMPANY_SECRET  # This will inject an environment variable with the name COMPANY_SECRET and with the value of the secret.
```

##### Template imported from a separate project
```yaml
# main project ame.yml
project: xgboost_project
tasks:
  - name: train_my_model
    fromTemplate: 
      name: xgboost_resources
      project: shared_templates
    executor:
      !poetry
      pythonVersion: 3.11
      command: python train.py

# other project ame.yml
project: shared_templates
templates:
  - name: xgboost_resources # Note that this is the name of the template
    resources:
      memory: 10G 
      cpu: 4 
      storage: 30G 
      nvidia.com/gpu: 1 
    secrets:
     - !ame # Secret stored by AME
       key: blob_storage_key # Key identifying a secret in AME's secret store.
       injectAs: MY_BLOB_STORAGE_SECRET # This will inject an environment variable with the name MY_BLOB_STORAGE_SECRET and with the value of the secret.

     # TODO this does not cover all vault cases
     - !vault # Secret stored in Vault
       vault: company_vault # Name of the vault to use.
       key: path/to/secret # Path to the secret in Vault.
       injectAs: COMPANY_SECRET  # This will inject an environment variable with the name COMPANY_SECRET and with the value of the secret.
```
###### Example notes

See the (section)[] on importing from other projects for more details.

### Importing from other projects

### Task input and output data (artifacts and saving)

Tasks can load data using [Datasets](todo) and save artifacts to the object storage AME is configured too use.

#### Quick examples

##### Save data in paths to object storage
 
```yaml
name: artifact_example
tasks:
  - name: artifact_task
    executor:
      !pipEnv
      command: python artifacts.py
    artifacts:
      paths:
        - path/to/artifact_dir
            
```

##### Automatically store new or changed files as artifacts
 
```yaml
name: artifact_example
tasks:
  - name: artifact_task
    executor:
      !pipEnv
      command: python artifacts.py
    artifacts:
      saveChangedFiles: true
            
```

##### Load data from a dataset
 
```yaml
name: artifact_example
tasks:
  - name: artifact_task
    executor:
      !pipEnv
      command: python artifacts.py
    dataSets:
      - ref:
          name: somedataset
          project: anotherproject
    artifacts:
      saveChangedFiles: true
            
```


### Task reference

Tasks can reference other Tasks. This is intended for usecases where inlining a Task is cumbersome for example when adding a model training Task or a Task pipeline. 

#### Quick examples

##### Referencing a Task for model training

```yaml
# main project ame.yml
project: xgboost_project
models:
  - name: product_recommendor
    training:
      task: # Remember this field expects a Task
        taskRef: train_my_model # This is considered a complete Task
tasks:
  - name: train_my_model
    fromTemplate: shared_templates.xgboost_resources
    executor:
      !poetry
      pythonVersion: 3.11
      command: python train.py
    resources:
      memory: 10G 
      cpu: 4 
      storage: 30G 
      nvidia.com/gpu: 1 
```

Alternatively if we were to inline the Task it would look like:

```yaml
# main project ame.yml
project: xgboost_project
models:
  - name: product_recommendor
    training:
      task:
        name: train_my_model
        executor:
          !poetry
          pythonVersion: 3.11
          command: python train.py
        resources:
          memory: 10G 
          cpu: 4 
          storage: 30G 
          nvidia.com/gpu: 1 
```

##### Referencing a Task from a Task
```yaml
# main project ame.yml
project: xgboost_project
models:
  - name: product_recommendor
    training:
      task:
        taskRef: train_my_model 
tasks:
  - name: other_task # TODO: how do we handle names for Tasks with a reference.
    taskRef: train_my_model
  - name: train_my_model
    fromTemplate: shared_templates.xgboost_resources
    executor:
      !poetry
      pythonVersion: 3.11
      command: python train.py
    resources:
      memory: 10G 
      cpu: 4 
      storage: 30G 
      nvidia.com/gpu: 1 
```

**Currently it is not supported to reference a Task from a different project but that is likely to be implemented in some form in the near future** 

### Triggering Tasks

For some usecases it might be required to automatically kick off a Task. If you are working with a high level construct such as a dataset or Model we recommended that you first check
if it has a mechanism that suits your needs as AME can often help you do less work that way. For example a Model automatically keep deployment up to date and trigger validation before 
deployming any new versions. Doing this manually would be non trivial. If you have a need not already covered read on.

The main mechanism of triggering independent Tasks is time based. We will be implementing more triggers for example git based triggers in the coming release.

In order to have a Task triggered on some recurring basis two things are required. A project with that Task must be present in an AME cluster and the desired Task must have the trigger field configured.

#### Quick examples

##### Trigger Task with cron schedule

```yaml
# main project ame.yml
project: xgboost_project
tasks:
  - name: my_task
    taskRef: train_my_model
  - name: train_my_model
    fromTemplate: shared_templates.xgboost_resources
    triggers:
      schedule: ***** # TODO some cron schedule
    executor:
      !poetry
      pythonVersion: 3.11
      command: python train.py
    resources:
      memory: 10G 
      cpu: 4 
      storage: 30G 
      nvidia.com/gpu: 1 
```

### DAGS

### Working with Tasks

### Pipelines

A task can consist of sub tasks.


#### Quick examples

##### Pipeline with inline sequential tasks


```yaml
# main project ame.yml
project: xgboost_project
tasks:
  - name: other_task # TODO: how do we handle names for Tasks with a reference.
    taskRef: train_my_model
  - name: train_my_model
    pipeline:
      - name: preprocessing 
        executor:
          !poetry
          pythonVersion: 3.11
          command: python prepare_data.py
        resources:
          memory: 4G 
          cpu: 2 
          storage: 30G 
      - name: training
        executor:
          !poetry
          pythonVersion: 3.11
          command: python train.py
        resources:
          memory: 10G 
          cpu: 4 
          storage: 30G 
          nvidia.com/gpu: 1 
```

##### Pipeline with equential tasks referenced

```yaml
# main project ame.yml
project: xgboost_project
tasks:
  - name: other_task # TODO: how do we handle names for Tasks with a reference.
    taskRef: train_my_model
  - name: train_my_model
    pipeline:
      - taskref: preprocessing
      - taskref: training
  - name: preprocessing
      executor:
        !poetry
        pythonVersion: 3.11
        command: python prepare_data.py
      resources:
        memory: 4G 
        cpu: 2 
        storage: 30G 
  - name: training
    executor:
      !poetry
      pythonVersion: 3.11
      command: python train.py
    resources:
      memory: 10G 
      cpu: 4 
      storage: 30G 
      nvidia.com/gpu: 1 

```

##### Pipeline with parallel tasks


```yaml
# main project ame.yml
project: xgboost_project
tasks:
  - name: other_task # TODO: how do we handle names for Tasks with a reference.
    taskRef: train_my_model
  - name: train_my_model
    pipeline:
      - - taskref: prepare_dataset_one 
        - taskref: prepare_datast_two 
      - taskref: training
  - name: prepare_dataset_one
      executor:
        !poetry
        pythonVersion: 3.11
        command: python prepare_data_one.py
      resources:
        memory: 4G 
        cpu: 2 
        storage: 30G 
  - name: prepare_dataset_two
      executor:
        !poetry
        pythonVersion: 3.11
        command: python prepare_data_two.py
      resources:
        memory: 4G 
        cpu: 2 
        storage: 30G 
  - name: training
    executor:
      !poetry
      pythonVersion: 3.11
      command: python train.py
    resources:
      memory: 10G 
      cpu: 4 
      storage: 30G 
      nvidia.com/gpu: 1 

```

### Environment variables

Environment variable can be set with the `environment` field. The field accepts a list of key value pairs, see the example below.

**Note** that there are a few environment variables set by AME, future releases of AME will return an error if you attempt to override them, see this (issue)[]. 

```shell
# Provided environment variables
# TODO fill this out
MLFLOW...
  
```

**Quick example**

```yaml
# ame.yml

tasks:
  - name: train_my_model
    executor:
      !poetry
      pythonVersion: 3.11
      command: python train.py
    resources:
      memory: 10G # 10 Gigabyte ram
      cpu: 4 # 4 CPU threads
      storage: 30G # 30Gigabyte of disk space
      nvidia.com/gpu: 1 # 1 Nvidia GPU
    environment:
      # inject environment variable SOME_VAR=SOME_VAL
      - key: SOME_VAR
        val: SOME_VAL
```

### Techinical details of Tasks
