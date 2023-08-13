# ame

![Version: 0.1.0-alpha1](https://img.shields.io/badge/Version-0.1.0--alpha1-informational?style=flat-square) ![Type: application](https://img.shields.io/badge/Type-application-informational?style=flat-square) ![AppVersion: 0.1.0-alpha1](https://img.shields.io/badge/AppVersion-0.1.0--alpha1-informational?style=flat-square)

A helm chart AME the artificial MLOps engineer.

## Requirements

| Repository | Name | Version |
|------------|------|---------|
| https://argoproj.github.io/argo-helm | argo-workflows | 0.32.1 |
| oci://registry-1.docker.io/bitnamicharts | minio | 12.6.12 |

## Values

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| ameVersion | string | `""` | Version used to select the images for AME's server, controller and default images Tasks and model deployments. |
| argo-workflows.enabled | bool | `true` |  |
| controller.autoscaling.enabled | bool | `false` |  |
| controller.image.repository | string | `"ghcr.io/teainspace/ame-controller/main"` |  |
| controller.image.tag | string | `"d162"` |  |
| controller.labels | object | `{}` |  |
| controller.logging.env_filter | string | `"info,controller=debug"` |  |
| controller.name | string | `"controller"` |  |
| controller.podSecurityContext | object | `{}` |  |
| controller.replicaCount | int | `1` |  |
| controller.service.port | int | `80` |  |
| controller.service.type | string | `"ClusterIP"` |  |
| controller.serviceAccount.create | bool | `true` |  |
| controller.serviceAccount.name | string | `"ame-controller"` |  |
| crds.install | bool | `true` | Flag for installing custom resource definitions with this see, see a discussion on the tradeoffs [here](TODO). |
| minio.enabled | bool | `true` |  |
| mlflow.endpoint | string | `"http://mlflow.ame-system.svc.cluster.local:5000"` |  |
| models.deployments.affinity | object | `{}` |  |
| models.deployments.autoscaling.enabled | bool | `false` |  |
| models.deployments.autoscaling.maxReplicas | int | `100` |  |
| models.deployments.autoscaling.minReplicas | int | `1` |  |
| models.deployments.autoscaling.targetCPUUtilizationPercentage | int | `80` |  |
| models.deployments.ingress.defaultIngress | string | `""` |  |
| models.deployments.ingress.host | string | `""` |  |
| models.deployments.nodeSelector | object | `{}` |  |
| models.deployments.resources | object | `{}` |  |
| models.deployments.resources | string | `nil` |  |
| models.deployments.securty.podSecurityContext | object | `{}` |  |
| models.deployments.securty.securityContext | object | `{}` |  |
| models.deployments.serviceAccount | object | `{"annotations":{},"create":true,"name":"ame-model"}` | The service account used by Tasks with minimal permissions. |
| models.deployments.tolerations | list | `[]` |  |
| namespace | object | `{"create":true,"name":"ame-system"}` | The namepace AME will operate within, this includes any depencies like Argo workflows, minio and keycloak. |
| objectStorage.s3.accessIdKey | string | `"root-user"` |  |
| objectStorage.s3.accessSecretKey | string | `"root-password"` |  |
| objectStorage.s3.bucket | string | `"ameprojectstorage"` |  |
| objectStorage.s3.endpoint | string | `"http://ame-minio:9000"` |  |
| objectStorage.s3.secretName | string | `"ame-minio"` |  |
| server.autoscaling.enabled | bool | `false` |  |
| server.image.repository | string | `"ghcr.io/teainspace/ame-server/main"` |  |
| server.image.tag | string | `"d162"` |  |
| server.ingress.annotations."cert-manager.io/cluster-issuer" | string | `"selfsigned-cluster-issuer"` |  |
| server.ingress.annotations."nginx.ingress.kubernetes.io/backend-protocol" | string | `"GRPC"` |  |
| server.ingress.annotations."nginx.ingress.kubernetes.io/cors-allow-headers" | string | `"DNT, Keep-Alive, User-Agent, X-Requested-With, If-Modified-Since, Cache-Control, Content-Type, Range, Authorization, x-grpc-web"` |  |
| server.ingress.annotations."nginx.ingress.kubernetes.io/cors-allow-origin" | string | `"*"` |  |
| server.ingress.annotations."nginx.ingress.kubernetes.io/enable-cors" | string | `"false"` |  |
| server.ingress.className | string | `"nginx"` |  |
| server.ingress.enabled | bool | `true` |  |
| server.ingress.hosts[0].host | string | `"ame.local"` |  |
| server.ingress.hosts[0].http.paths[0].backend.service.name | string | `"ame-server-service"` |  |
| server.ingress.hosts[0].http.paths[0].backend.service.port.number | int | `3342` |  |
| server.ingress.hosts[0].http.paths[0].path | string | `"/"` |  |
| server.ingress.hosts[0].http.paths[0].pathType | string | `"ImplementationSpecific"` |  |
| server.ingress.tls[0].hosts[0] | string | `"ame.local"` |  |
| server.ingress.tls[0].secretName | string | `"ame-tls-cert"` |  |
| server.labels | object | `{}` |  |
| server.name | string | `"server"` |  |
| server.podSecurityContext | object | `{}` |  |
| server.replicaCount | int | `1` |  |
| server.resources | object | `{}` |  |
| server.service.port | int | `3342` |  |
| server.service.type | string | `"ClusterIP"` |  |
| server.serviceAccount.create | bool | `true` |  |
| server.serviceAccount.name | string | `"ame-server"` |  |
| task.affinity | object | `{}` |  |
| task.image.repository.repository | string | `"ghcr.io/teainspace/ame-controller/main"` |  |
| task.image.repository.tag | string | `"d162"` |  |
| task.nodeSelector | object | `{}` |  |
| task.resources | string | `nil` |  |
| task.securty.podSecurityContext | object | `{}` |  |
| task.securty.securityContext | object | `{}` |  |
| task.serviceAccount | object | `{"annotations":{},"create":true,"name":"ame-task"}` | The service account used by Tasks with minimal permissions. |
| task.tolerations | list | `[]` |  |

----------------------------------------------
Autogenerated from chart metadata using [helm-docs v1.11.0](https://github.com/norwoodj/helm-docs/releases/v1.11.0)
