# Configuring AME

AME attempts to keep configuration in a single config map, which both the server and controller will read from. 
There options are then exposed via helm and kustomize as well. This page will walk through AME configuration options.

Note that AME has a top down configuration approach, where cluster wide defaults are set in the in the config map, project wide defaults are 
set on a per project basis.

## Default ingress

If you are deploying models it is important to set a default ingress as AME can not yet generate a good default.

This is set like ... TODO

Options for other ingress than nginx?

## Namespace

AME operates within a single namespace, this defaults to ame-system. To override this ... TODO

## Task service account

AME uses a service account for Tasks with minimal permissions. By default this is `ame-task`, to override this ... TODO

## Default images

AME provides a number of images with good defaults for exeuting Tasks and deploying models. These can all be overridden ... TODO

## Object storage

AME uses object storage to store and cache various files. There are a number of config options that can be set here.
For example if you don't want object storage deployed a long side ame but separate from the cluster.

TODO how?

## Opentelemtry

TODO

### Logs

## Mlflow

## keycloak

## Argo Workflows


## Promethus

## Controller specifics

## Server specifics
