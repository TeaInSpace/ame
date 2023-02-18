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
