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

There are multiple

#### Mlflow



#### MLserver

#### Kserve

#### Model validation

#### Model monitoring

### Batch inference
