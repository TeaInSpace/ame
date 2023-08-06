# Models

Models are one of AME's higher level constructs, see what that means [here](). if you are configuring how a model should be trained, deployed, monitored or validated this is the right place.
Models exist in an AME file along side Datasets Tasks and Templates.

### Model training

Model training is configured described use a [Task](tasks.md).

AME can be deployed with a an MLflow instance which will be exposed to the Training Task allowing for simple storage and retrievel of models metrics and experiments.


```yaml
# main project ame.yml
project: xgboost_project
models:
  - name: product_recommendor
    training:
      task: 
        taskRef: train_my_model 
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

### Model deployment 

If AME is setup with a model reigstry see supported registries [here](todo) models can be deployed for inference.

Just like for [`Tasks`](task), you can and probably should define the resources required to perform inference with your model.
Here are configuration examples with the serving options available with AME.

#### Mlflow

```yaml
# main project ame.yml
project: xgboost_project
models:
  - name: product_recommendor
    training:
      task: 
        taskRef: train_my_model 
    deployment:
      resources:
        memory: 10G
        cpu: 4
        storage: 10G
        nvidia.com/gpu: 1
      autoDeploy: true
tasks:
  ...
```

#### MLserver

Mlserver support is planned for a future release, see [issue](todo)

#### Kserve

Mlserver support is planned for a future release, see [issue](todo)

##### Triton

#### Advanced deployment configuration

##### Ingress

If you are hosting AME your self, there are a number of decisions that need to be made with regards to mode deployment. Currently AME does not automatically generate an ingress
configuration and therefore one must be provided either. You can provide a cluster wide default as well as individual override for models.

See how to set a cluster wide default [here](clusterwideingress).

Model specific ingress can be set here. The plan is to provide better abstractions to avoid having to work with this directly, see [this](ingressabstractionissue).

Setting model deployment ingress:


```yaml
# main project ame.yml
project: xgboost_project
models:
  - name: product_recommendor
    deployment:
      ingressAnnotations:
        TODO
      resources:
        memory: 10G
        cpu: 4
        storage: 10G
        nvidia.com/gpu: 1
      autoDeploy: true
tasks:
  ...
```

##### Replicas

For productioon deploiyments you will likely want some degree of relication for model instances. Custer wide defaults can be set [here](todo).
Model specific replicas can set like this:

```yaml
# main project ame.yml
project: xgboost_project
models:
  - name: product_recommendor
    deployment:
      replicas: 3
      ...
tasks:
  ...
```

##### Image

If AME's default deployment image is insufficient for your use case a custom image can be set. This can be change cluster wide [here](todo).

Model specific deployment images can be set like this:

```yaml
# main project ame.yml
project: xgboost_project
models:
  - name: product_recommendor
    deployment:
      image: my.deployment.image
      ...
tasks:
  ...
```

If a secret is required to access the image remember to provide that secret to AME, a guide is [here]().

### Model validation

AME supports validating models versions before they are deployed. To enable this we have to provide a task that will succeed or fail based on the validity of a model version.
See a guide [here]().

```yaml
# main project ame.yml
project: xgboost_project
models:
  - name: product_recommendor
    validation:
      task:
        taskRef: model_validation
tasks:
  ...

```

#### Model monitoring

### Batch inference

TODO add a reference with all objects and options.
