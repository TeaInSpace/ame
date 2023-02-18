CONTROLLER_IMAGE := "ame-controller"
SERVER_IMAGE := "ame-server"
EXECUTOR_IMAGE := "ame-executor"
ORG := "teainspace"
AME_REGISTRY_PORT := `docker port main | grep -o ':.*' | grep -o '[0-9]*' || true`
IMG_TAG := ":latest"
AME_REGISTRY := "main.localhost:" + AME_REGISTRY_PORT
SERVER_IMAGE_TAG := AME_REGISTRY + "/" +  SERVER_IMAGE + IMG_TAG
EXECUTOR_IMAGE_TAG := AME_REGISTRY + "/" +  EXECUTOR_IMAGE + IMG_TAG
LOCAL_EXECUTOR_IMAGE_TAG := "main:" + AME_REGISTRY_PORT + "/" + EXECUTOR_IMAGE + ":latest"
CONTROLLER_IMAGE_TAG := AME_REGISTRY + "/" +  CONTROLLER_IMAGE + IMG_TAG
TARGET_NAMESPACE := "ame-system"
TASK_SERVICE_ACCOUNT := "ame-task"
AME_HOST := "ame.local"
KEYCLOAK_HOST := "keycloak.ame.local"

# See https://github.com/rust-lang/rustfix/issues/200#issuecomment-923111872
export __CARGO_FIX_YOLO := "1"

default:
  @just --list --unsorted --color=always | rg -v "    default"

setup_toolchains:
 rustup toolchain install nightly 

tools:
  rustup component add clippy --toolchain nightly
  rustup component add rustfmt --toolchain nightly
  cargo install --locked cargo-spellcheck
  cargo install --locked typos-cli
  cargo install --locked cargo-audit
  cargo install --locked cargo-outdated
  cargo install --locked cargo-udeps

fix: fmt
  cargo fix --workspace --allow-dirty --tests --allow-staged
  cargo +nightly clippy --workspace --fix --allow-dirty --allow-staged --tests --all -- -D warnings
  typos --exclude **/primer.css ./ -w
  cargo spellcheck fix

fmt:
  cargo +nightly fmt 

check:
  cargo spellcheck check 
  typos --exclude **/primer.css ./
  cargo audit
  cargo +nightly fmt --check 
  cargo +nightly clippy --workspace --tests --all -- -D warnings
  # cargo outdated --exclude leptos
  cargo +nightly udeps --all-targets --workspace --show-unused-transitive --exclude web # TODO: solve false positives for web package.

test *ARGS:
  cargo test --workspace {{ARGS}}

test_controller *ARGS:
  cargo test -p controller {{ARGS}}

test_cli *ARGS:
  cargo test -p cli {{ARGS}}

test_server *ARGS:
  cargo test -p service {{ARGS}}

server_logs *ARGS:
  kubectl logs -n {{TARGET_NAMESPACE}} -l app=ame-server {{ARGS}}

controller_logs *ARGS:
  kubectl logs -n {{TARGET_NAMESPACE}} -l app=ame-controller {{ARGS}}

watch_web:
  cargo leptos watch

review_snapshots:
  cargo insta review

crdgen:
 cargo run --bin crdgen > manifests/crd.yaml
 cargo run --bin project_src_crdgen > manifests/project_src_crd.yaml
 cargo run --bin project_crdgen > manifests/project_crd.yaml

start_controller:
  cargo run --bin controller

start_server:
  #!/bin/sh
  export S3_ENDPOINT=http://$(kubectl get svc -n {{TARGET_NAMESPACE}} ame-minio -o jsonpath='{.status.loadBalancer.ingress[0].ip}'):9000
  export S3_ACCESS_ID=minio
  export S3_SECRET=minio123
  cargo run --bin ame-server

run_cli *ARGS:
 cargo run -p cli {{ARGS}}

setup_cli:
 cargo run -p cli setup http://$(kubectl get svc -n {{TARGET_NAMESPACE}} ame-server-service -o jsonpath='{.status.loadBalancer.ingress[0].ip}'):3342

setup_cli_ingress:
 cargo run -p cli setup https://ame.local:$(kubectl get service -n ingress-nginx ingress-nginx-controller -o jsonpath='{.spec.ports[1].nodePort}')


# Local cluster utilities

setup_cluster: k3s create_namespace create_service_accounts install_cert_manager install_argo_workflows deploy_keycloak deploy_minio deploy_nginx

k3s:
  k3d cluster create main \
    --servers 1 \
    --registry-create main \
    --k3s-arg "--disable=traefik@server:*" \
    --k3s-arg '--kubelet-arg=eviction-hard=imagefs.available<0.1%,nodefs.available<0.1%@agent:*' \
    --k3s-arg '--kubelet-arg=eviction-minimum-reclaim=imagefs.available=0.1%,nodefs.available=0.1%@agent:*' \
    --image rancher/k3s:v1.25.5-rc2-k3s1

create_namespace:
  kubectl create ns {{TARGET_NAMESPACE}}

create_service_accounts:
  kubectl create sa {{TASK_SERVICE_ACCOUNT}} -n {{TARGET_NAMESPACE}} # TODO: move this to vault setup

delete_cluster:
  k3d cluster delete main

install_crd:
 kubectl apply -f manifests/crd.yaml
 kubectl apply -f manifests/project_src_crd.yaml
 kubectl apply -f manifests/project_crd.yaml

set_host_entries:
 #!/bin/sh
 LB_IP=$(kubectl get svc -n ingress-nginx ingress-nginx-controller -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
 
 echo "In order to test AME with SSL a host entry must be set in your local host file, this requires root permissions"

 just ensure_host_entry $LB_IP {{AME_HOST}}
 just ensure_host_entry $LB_IP {{KEYCLOAK_HOST}}

ensure_host_entry IP HOST:
 #!/bin/sh
 echo "ensure host entry: {{IP}} {{HOST}}"
 if grep -q {{HOST}} "/etc/hosts"; then
 echo "Found existing ame host entry, will replace the IP to make sure it is up to date."
 sudo sed -i "s/.* {{HOST}}.*/{{IP}} {{HOST}}/" /etc/hosts
 else
 echo "Adding new entry to entry/hosts"
 sudo sed -i '$a\ {{IP}} {{HOST}}' /etc/hosts
 fi

install_argo_workflows:
  kubectl apply -n {{TARGET_NAMESPACE}} -f https://raw.githubusercontent.com/argoproj/argo-workflows/master/manifests/quick-start-postgres.yaml

deploy_ame_to_cluster: install_crd build_and_deploy_server build_and_deploy_controller build_and_push_executor_image

build_and_deploy_server: build_server_image push_server_image deploy_server

build_and_deploy_controller: build_controller_image push_controller_image deploy_controller 

build_and_push_executor_image: build_executor push_executor_image

deploy_server:
  #!/bin/sh
  SERVER_IMAGE_TAG="main:{{AME_REGISTRY_PORT}}/{{SERVER_IMAGE}}:latest"
  cd ./manifests/server/local/
  kustomize edit set image ame-server=$SERVER_IMAGE_TAG
  kustomize edit set namespace {{TARGET_NAMESPACE}}
  kustomize build . | kubectl apply -f -
  sleep 1
  kubectl delete pod -n ame-system -l app=ame-server
  kubectl wait pods -n {{TARGET_NAMESPACE}} -l app=ame-server --for condition=Ready --timeout=90s

deploy_server_to_ask:
  #!/bin/sh
  cd ./manifests/server/aks/
  kustomize edit set namespace {{TARGET_NAMESPACE}}
  kustomize build . | kubectl apply -f -
  sleep 1
  kubectl delete pod -n ame-system -l app=ame-server
  kubectl wait pods -n {{TARGET_NAMESPACE}} -l app=ame-server --for condition=Ready --timeout=90s

deploy_controller_to_ask:
  #!/bin/sh
  cd ./manifests/controller/aks/
  kustomize edit set namespace {{TARGET_NAMESPACE}}
  kustomize build . | kubectl apply -f -
  sleep 1
  kubectl delete pod -n ame-system -l app=ame-server
  kubectl wait pods -n {{TARGET_NAMESPACE}} -l app=ame-server --for condition=Ready --timeout=90s


deploy_controller:
  #!/bin/sh
  CONTROLLER_IMAGE_TAG="main:{{AME_REGISTRY_PORT}}/{{CONTROLLER_IMAGE}}:latest"
  echo $CONTROLLER_IMAGE_TAG
  cd ./manifests/controller/local/
  cat <<'EOF' > controller_config.yaml
  apiVersion: v1
  kind: ConfigMap
  metadata:
    name: ame-controller-configmap
  data:
    executor_image: {{LOCAL_EXECUTOR_IMAGE_TAG}}
  EOF
  kustomize edit set image ame-controller=$CONTROLLER_IMAGE_TAG
  kustomize edit set namespace {{TARGET_NAMESPACE}}
  kustomize build . | kubectl apply -f -
  sleep 1
  kubectl delete pod -n ame-system -l app=ame-controller
  kubectl wait pods -n {{TARGET_NAMESPACE}} -l app=ame-controller --for condition=Ready --timeout=90s


# Container images

publish_images: build_controller_image build_server_image build_executor push_server_image push_controller_image push_executor_image

build_controller_image:
  docker build . -f Dockerfile.controller -t {{CONTROLLER_IMAGE_TAG}}

build_server_image:
  docker build . -f Dockerfile.server -t {{SERVER_IMAGE_TAG}}

build_executor: ## Builder docker image for the server.
	docker build ./executor/ -t {{EXECUTOR_IMAGE_TAG}}

push_server_image: 
  docker push {{SERVER_IMAGE_TAG}}

push_executor_image:
  docker push {{EXECUTOR_IMAGE_TAG}}

push_controller_image:
  docker push {{CONTROLLER_IMAGE_TAG}}

remove_server:
	kustomize build manifests/server/local | kubectl delete --ignore-not-found=true -f -
 
describe_server:
  kubectl describe pod -l app=ame-server -n {{TARGET_NAMESPACE}}

install_commit_template:
  git config commit.template ./.git_commit_message_template

start_opentelemtry_collector:
  docker run -d -p6831:6831/udp -p6832:6832/udp -p16686:16686 jaegertracing/all-in-one:latest

run_client:
  cargo run --bin ame-client

deploy_minio:
  #!/bin/sh
  cd ./manifests/minio/base/resources
  kustomize edit set namespace {{TARGET_NAMESPACE}}
  kustomize build . | kubectl apply -f -
  sleep 2
  kubectl wait pods -n {{TARGET_NAMESPACE}} -l app=ame-minio --for condition=Ready --timeout 90s

install_keycloak_operator:
  kubectl apply -n {{TARGET_NAMESPACE}} -f https://raw.githubusercontent.com/keycloak/keycloak-k8s-resources/20.0.1/kubernetes/keycloaks.k8s.keycloak.org-v1.yml
  kubectl apply -n {{TARGET_NAMESPACE}} -f https://raw.githubusercontent.com/keycloak/keycloak-k8s-resources/20.0.1/kubernetes/keycloakrealmimports.k8s.keycloak.org-v1.yml
  kubectl apply -n {{TARGET_NAMESPACE}} -f https://raw.githubusercontent.com/keycloak/keycloak-k8s-resources/20.0.1/kubernetes/kubernetes.yml

add_helm_repos:
  helm repo add oauth2-proxy https://oauth2-proxy.github.io/manifests
  helm repo add ingress-nginx https://kubernetes.github.io/ingress-nginx
  helm repo add bitnami https://charts.bitnami.com/bitnami
  helm repo add ncsa https://opensource.ncsa.illinois.edu/charts/
  helm repo add hashicorp https://helm.releases.hashicorp.com
  helm repo update

deploy_keycloak:
  helm install keycloak bitnami/keycloak --set auth.adminPassword=admin -f ./manifests/keycloak/values_local.yaml

deploy_keycloak_ask:
  helm install keycloak bitnami/keycloak --set auth.adminPassword=admin -f ./manifests/keycloak/values.yaml

deploy_oauth2_proxy:
  helm install oauth2-proxy oauth2-proxy/oauth2-proxy \
    -f ./manifests/oauth2-proxy/oauth2proxy-values.yaml --wait

deploy_oauth2_proxy_local:
  helm install oauth2-proxy oauth2-proxy/oauth2-proxy \
    -f ./manifests/oauth2-proxy/oauth2proxy-values-local.yaml --wait

deploy_mlflow:
  helm install mlflow ncsa/mlflow

deploy_nginx:
  helm upgrade --install ingress-nginx ingress-nginx \
    --repo https://kubernetes.github.io/ingress-nginx \
    --namespace ingress-nginx --create-namespace 

setup_ask:
  # ask ingress nginx install: https://kubernetes.github.io/ingress-nginx/deploy/#azure
  kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/controller-v1.5.1/deploy/static/provider/cloud/deploy.yaml

  # Standard cert manager install: https://cert-manager.io/docs/installation/
  kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.11.0/cert-manager.yaml
  kubectl apply -f ./manifests/cert-manager/issuers.yaml

deploy_vault:
  helm install vault hashicorp/vault \
      --set "server.dev.enabled=true" -n vault --create-namespace
  just configure_vault_k8s_auth
  just set_vault_secret

configure_vault_k8s_auth:
   #!/bin/sh
   kubectl exec -n vault -it vault-0 -- vault auth enable kubernetes
   kubectl exec -n vault -it vault-0 -- bin/sh -c 'vault write auth/kubernetes/config \
      kubernetes_host="https://$KUBERNETES_PORT_443_TCP_ADDR:443"'
   
   kubectl exec -n vault -it vault-0 -- vault policy write {{TASK_SERVICE_ACCOUNT}} - <<EOF
   path "internal/data/database/config" {
   capabilities = ["read"]
   }
   EOF
   
   kubectl exec -n vault -it vault-0 -- vault write auth/kubernetes/role/{{TASK_SERVICE_ACCOUNT}} \
   bound_service_account_names={{TASK_SERVICE_ACCOUNT}} \
   bound_service_account_namespaces={{TARGET_NAMESPACE}} \
   policies=ame-task \
   ttl=24h

set_vault_secret:
   #!/bin/sh
   kubectl exec -n vault -it vault-0 -- vault secrets enable -path=internal kv-v2
   kubectl exec -n vault -it vault-0 -- vault kv put internal/data/database/config username="db-readonly-username" password="db-secret-password"

install_cert_manager:
  kubectl create ns cert-manager --dry-run=client -o yaml | kubectl apply -f -
  kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.9.1/cert-manager.yaml
  sleep 20 
  kubectl wait --for=condition=Ready pods --all --namespace cert-manager
  kubectl apply -f manifests/cert-manager/self_signed_issuer.yaml

add_mc_alias:
  mc alias set minio http://$(kubectl get service ame-minio -n ame-system -o jsonpath='{.status.loadBalancer.ingress[0].ip}'):9000 minio minio123

start_chromium_without_sec:
	chromium --disable-web-security --user-data-dir=$HOME/Downloads --ignore-certificate-errors
