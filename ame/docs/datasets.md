# Data Sets

AME has a builtin notion of data sets in allowing user's to think in terms of data sets and not just raw tasks. 

Here is an example of what a simple data set configuration looks like:

```yaml
# ame.yaml
...
dataSets:
  - name: mnist
    path: ./data # Specifies where the tasks stores data.
#    task:
      taskRef: fetch_mnist # References a task which produces data.     
```

In the simplest form a data set is just a [task](todo) which produces data a long with a storage mechanism. This allows for a number of benefits over just using tasks directly.
Data can be produced once and used many times, for example if a number of tasks are scheduled AME can prepare the dataset once and use it across all of the dependent tasks.

### Configuring a data set

A simple data set cfg is quick to set and can then be progressively enhanced as your needs expand. Here we will walk through the process of first setting up a simple data set
and then go through the more advanced options.

The minimum requirements for a dataset is a `path` pointer to where data should be saved from and a `Task` which will produce data at that path. As shown in the mnist example above.
Lets start with that here:

```yaml
# ame.yaml
...
dataSets:
  - name: mnist
    path: ./data # Specifies where the tasks stores data.
#    task:
      taskRef: fetch_mnist # References a task which produces data.     
```

So far so good, we have a path `data` and reference a `Task` that produces our data.


### Interacting with data sets

To see the status of live data sets, use the AME's cli. Current it is only possible to see data sets that are in use, meaning referenced by some running task.

```bash
ame ds list
```  
 