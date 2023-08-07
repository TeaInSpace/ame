# Introduction

AME is an MLOps platform built around [Kubernetes](). Although as a user you should not have to interact with Kubernetes directly or now anything about Kubernetes. We
highlight the fact since it provides a number of advantages. AME can be deployed anywhere Kubnerets can, which essentially means everywhere from onprem self managed to 
a fully managed Kubernetes solution in the cloud. It also means that AME can easily pull in different opensource components to expand functionality as needed. For example
we use [Argo workflows](todo) as the underlying worflow engine and mlfow as a model registry.

For administrators of AME it means that all of the existing Kubernetes knowlegde and tooling still applies and AME can be managed just like any other Kubernetes depoyment.

It also means that there is a non trivial amount of complexity invovled with standing up an AME instance and managing it, as you inhereit all of the complexities introduced
by Kubernetes. In many cases this is not a problem as even without Kubernetes you would need to deal with the same problems. However for simpler cases where a small
team just wants som job scheduling for automatic training Kubernetes introduced a lot of overhead. Especially if no one on the team is familiar with Kuberenetes. For now
we can not do much about, as we are focusing on production setups where Kubernetes is perfectly justifyable. However it is the goal to decouple AME from kubnernetes, which
will allow for simple setups as well.  

This section will take your for a practical tour of what AME's features with practical examples. The [user guide](userguide) will go more indepth on user facing topics and
the [operator manual] goes indepth on deploying and administrating AME.

We have an examples [directory](examples) with examples of TODO 
