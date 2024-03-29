# Guides

## From zero to live model

**This guide is focused on using AME** if you are looking for a deployment guide go [here](todo).



This guide will walk through going from zero to having a model served through an the [V2 inference protocol](https://docs.seldon.io/projects/seldon-core/en/latest/reference/apis/v2-protocol.html).
it will be split into multiple sub steps which can be consumed in isolation if you are just looking for a smaller guide on that specific step.

Almost any python project should be usable but if you want to follow along with the exact same project as the guide clone [this]() repo.

### Setup the CLI

Before we can initialise an AME project we need to install the ame [CLI](todo) and connect with your AME instance.

TODO describe installation

### Initialising AME in your project

The first step will be creating an `ame.yaml` file in the project directory.

This is easiet to do with the ame [CLI]() by running `ame project init`. The [CLI]() will ask for a project and then produce a file
that looks like this:

```yaml
projectName: sklearn_logistic_regression
```

### The first training

Not very exciting but it is a start. Next we want to set up our model to be run by AME. The most important thing here is the Task that will train the model so
lets start with that.

Here we need to consider a few things, what command is used to train a model, how are dependencies managed in our project, what python version do we need and
how many resources does our model training require.

If you are using the [repo]() for this guide, you will want a task configured as below. 

```yaml

projectid: sklearn_logistic_regression
tasks:
  - name: training
    !poetry
    executor:
      pythonVersion: 3.11
      command: python train.py
    resources:
      memory: 10G 
      cpu: 4 
      storage: 30G 
      nvidia.com/gpu: 1 
```

## Your first Task

[`Tasks`](TODO) are an important building block for AME. This guide will walk you through the basic of constructing and running [`Task`](todo). 

We assume that the AME [CLI](todo) is setup and connected to an AME instance. If not see this [guide](todo). 

Before we can run a task we must have a project setup. To init a project follow the commands as shown below, replacing myproject with the 
path to your project.

```sh
cd myproject
ame init
```

Now you should have an AME file ame.yaml inside your project:
```yaml
name: myproject
```

Not very exciting yet. Next we want to add a Task to this file so we can run it.
Update your file to match the changes shown below.

```yaml
name: myproject
tasks:
  - name: training
    !poetry
    executor:
      pythonVersion: 3.11
      command: python train.py
    resources:
      memory: 2G 
      cpu: 2 
      storage: 10G 
```

Here we add a list of tasks for our project, containing a single `Task` called training. Lets look at the anatomy of training.

First we set the name `name: training`, pretty standard YAML. Next we set the [executor](todo). This syntax might seem a bit confusing
if you have not used this YAML feature before. `!poetry` adds a tag to the executor indicating the executor type. In this case we are 
using the poetry executor. It requires two fields to be set. the Python version and the command to run. This tells AME how to execute the [`Task`](todo).

Finally we set the required resources. 2G ram, 2 cpu threads and 10G of storage.

To run the task we can use the CLI:
```sh
ame task run
```

 
 
## Validating models before deployment

To ensure that a new model versions perform well before exposing them AME supports model validation. This is done by providing AME with a `Task` which 
will succeed if the model passes validation and fail if not.

Example from [ame-demo](https://github.com/TeaInSpace/ame-demo):

```yaml

projectid: sklearn_logistic_regression
models:
  - name: logreg
    type: mlflow
    validationTask: # the validation task is set here.
      taskRef: mlflow_validation 
    training: 
      task:
        taskRef: training
    deployment:
      auto_train: true
      deploy: true
      enable_tls: false
tasks:
  - name: training
    projectid: sklearn_logistic_regression
    templateRef: shared-templates.logistic_reg_template
    taskType: Mlflow
  - name: mlflow_validation
    projectid: sklearn_logistic_regression
    runcommand: python validate.py
```

This approach allows for a lot of flexibility of how models are validated, at the cost of writing the validation your self. In the future AME will provide builtin options for common validation configurations as well, see the [roadmap](todo).

### Using MLflow metrics

Here we will walk through how to validate a model based on recorded metrics in MLflow, using the [ame-demo](https://github.com/TeaInSpace/ame-demo) repository as an example. The model is a simple logistic regresser, the training code looks like this:

```python
import numpy as np
from sklearn.linear_model import LogisticRegression
import mlflow
import mlflow.sklearn
import os

if __name__ == "__main__":
    X = np.array([-2, -1, 0, 1, 2, 1]).reshape(-1, 1)
    y = np.array([0, 0, 1, 1, 1, 0])
    lr = LogisticRegression()
    lr.fit(X, y)
    score = lr.score(X, y)
    print("Score: %s" % score)
    mlflow.log_metric("score", score)
    mlflow.sklearn.log_model(lr, "model", registered_model_name="logreg")
    print("Model saved in run %s" % mlflow.active_run().info.run_uuid)
```

Notice how the score is logged as a metric. We can use that in our validation.

AME exposes the necessary environment variables to running tasks so we can access the Mlflow instance during validation just by using the Mlflow library.

```python
TODO

```
