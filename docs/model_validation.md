# Validating models before deployment

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

Here we will walk through how to validate a model based on recorded metrics in MLflow, using the [ame-demo](https://github.com/TeaInSpace/ame-demo) repository as an example. The model is a simple logistic regressor, the training code looks like this:

```python
import numpy as np
from sklearn.linear_model import LogisticRegression
import mlflow
import mlflow.sklearn
import os

X = np.array([-2, -1, 0, 1, 2, 1]).reshape(-1, 1)
y = np.array([0, 0, 1, 1, 1, 0])
lr = LogisticRegression()
lr.fit(X, y)
score = lr.score(X, y)
mlflow.log_metric("score", score)
mlflow.sklearn.log_model(lr, "model", registered_model_name="logreg")
print("Model saved in run %s" % mlflow.active_run().info.run_uuid)
```

Notice how the score is logged as a metric. We can use that in our validation.

AME exposes the necessary environment variables to running tasks so we can access the Mlflow instance during validation just by using the Mlflow library.

**Note**: the model name is hard coded , in the future the model name will be made available as an environment variable allowing for better reusability of validation code.

```python
# validation.py
import sys
import mlflow
from mlflow.entities import ViewType

# We fetch the metrics for latest run for the logreg model.
models = mlflow.search_registered_models(filter_string="name='logreg'")
run_id = models[0].latest_versions[-1].run_id
run = mlflow.get_run(run_id)

if run.data.metrics['score'] < 0.6:
    sys.exit(1)

```

A validation task indicates failure with a non zero exit code. In this example if our model scores below 0.6 the task will exit with code 1 indicating a failure.

In this example we keep `validation.py` in the same repository as our training code, that is how ever not required. We could use a task from a completely separate project and share the validation logic between multiple projects.

Our AME file will look as follows:
```yaml

projectid: mlflow_validation_example
models:
  - name: logreg # Note the model name is what mlflow will use as well and the name used during validation.
    type: mlflow # This tells AME to train the model using mlfow.
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
    projectid: mlflow_validation_example
    taskType: Mlflow # Since this is an mlflow task, AME knows how to run it.
  - name: mlflow_validation
    projectid: mlflow_validation_example
    runcommand: python validate.py
```

See [ame-demo](https://github.com/TeaInSpace/ame-demo) for a full list of files.

We can add these files a git repository and add that repository as a project source in AME.

```bash
ame projectsrc create https://github.com/myuser/myrepo.git
```

Now you will be able to observe the model being trained, validated and then deployed.
