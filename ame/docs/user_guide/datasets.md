# Data Sets

AME has a builtin notion of data sets in allowing user's to think in terms of data sets and not just raw tasks. 

It is important to note that DataSets in AME should be treated as ephemeral and not long term storage. If AME is running
out of space any cached data can be deleted.

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
    task:
      taskRef: fetch_mnist # References a task which produces data.     
```

So far so good, we have a path `data` and reference a `Task` that produces our data.

#### Dataset size

If a dataset is large it is a good idea to specifiy the storage requirements. This will allow AME to warn you if the object storage is running out.

If you do not specify the size AME will attempt to save the dataset, detect the failure and then produce an alert.

```yaml
# ame.yaml
...
dataSets:
  - name: mnist
    path: ./data # Specifies where the tasks stores data.
    size: 50Gi
    task:
      taskRef: fetch_mnist # References a task which produces data.     
```


### Interacting with data sets

To see the status of live data sets, use the AME's cli. Current it is only possible to see data sets that are in use, meaning referenced by some running task.

```bash
ame dataset list
ame ds list # or shortend
```  

You can also view datasets from AME's dashboard:

TODO: dataset image

### Consuming data from object storage

AME does not yet have builtin support for extracing data from object storage, although it will in the near future, see the tracking issue [here](). 
It is still quite simplte to accomplish this in pure python, so we shall demonstrate that here.

