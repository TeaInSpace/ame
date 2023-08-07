# Quick Start

This page will get a basic Kubernets and AME setup deployed with minimal effort.

If you want skip setting up your own instance, we provide a test instance behind Github
login [here](todo).

## Kubernetes

There are a number of options for deploying a local cluster. The AME repository contains bootstrapping scripts
for the most common options. If you already have a cluster skip to the next step. If you are looking to deploy
a to the cloud see [this](todo).

## Deploying AME

AME distributed as a helm chart. Add the helm repository and install the chart. 

```sh
helm add ...
helm install ...
```
For a testing instance the defaults should be adequate. By default a minio instance will be included for object storage.

Keycload can be included for for user managment and authentication.

```sh
helm install ...withkeycloak
```

## First time setup

To get started we can configure AME either through the CLI, CLI or declaratively with a config map in the cluster. We will use the [CLI](todo).

Start by connecting the CLI to the new instance.

```sh
ame connect http://localhost:8003
```

**Note* that the port might be different, this will be evident in the helm chart output. On the first connect the CLI will check for any issues in the setup.
For example if there is no available object storage. There should be no issues to report at this point, otherwise please make an [issue](todo) or ask a question [here](todo).

If you think something has gone wrong you can always run `ame admin check` and AME will report any issues. The dashboard will also make it clear if there is any any need to take action.

Noe you are all setup to go through the walkthrough. If you have any questons or problem don't hestitate to ask. TODO where?
