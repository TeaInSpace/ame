CONTROLLER_NAME := "ame-controller"
SERVER_IMAGE := "ame-server"
EXECUTOR_NAME := "ame-executor"
ORG := "teainspace"
AME_REGISTRY_PORT := `echo $(IFS=: && list=($(docker port main)) && echo ${list[1]})`
AME_REGISTRY := "main.localhost:" + AME_REGISTRY_PORT
SERVER_IMAGE_TAG := AME_REGISTRY + "/" +  SERVER_IMAGE + ":latest"
TARGET_NAMESPACE := "ame-system"


# See https://github.com/rust-lang/rustfix/issues/200#issuecomment-923111872
export __CARGO_FIX_YOLO := "1"

default:
  @just --list --unsorted --color=always | rg -v "    default"

setup_toolchains:
 rustup toolchain install nightly 

tools:
  rustup component add clippy --toolchain nightly
  rustup component add rustfmt --toolchain nightly
  cargo install --locked cargo-insta
  cargo install --locked cargo-audit
  cargo install --locked cargo-outdated
  cargo install --locked cargo-spellcheck
  cargo install --locked typos-cli
  cargo install --locked cargo-chef

fix: fmt
  cargo fix --workspace --allow-dirty --tests --allow-staged
  cargo +nightly clippy --workspace --fix --allow-dirty --allow-staged --tests --all -- -D warnings
  typos controller/ -w
  cargo spellcheck fix

fmt:
  cargo +nightly fmt 

check: && test 
  cargo spellcheck check
  typos controller/
  cargo audit
  cargo +nightly fmt --check 
  cargo +nightly clippy --workspace --tests --all -- -D warnings
  cargo outdated

k3s:
  k3d cluster create main \
    --servers 1 \
    --registry-create main \
    --k3s-arg "--disable=traefik@server:*" \
    --k3s-arg '--kubelet-arg=eviction-hard=imagefs.available<0.1%,nodefs.available<0.1%@agent:*' \
    --k3s-arg '--kubelet-arg=eviction-minimum-reclaim=imagefs.available=0.1%,nodefs.available=0.1%@agent:*'

delete_cluster:
  k3d cluster delete main

test:
  cargo test --workspace

review_snapshots:
  cargo insta review

crdgen:
 cargo run --bin crdgen > manifests/crd.yaml

install_crd:
 kubectl apply -f manifests/crd.yaml

install_argo_workflows:
 kubectl apply -n {{TARGET_NAMESPACE}} -f manifests/argo.yaml

create_namespace:
  kubectl create ns {{TARGET_NAMESPACE}}

setup_cluster: k3s create_namespace install_crd install_argo_workflows deploy_keycloak deploy_minio

start_controller:
  cargo run --bin controller

start_server:
  #!/bin/sh
  export S3_ENDPOINT=http://$(kubectl get svc -n {{TARGET_NAMESPACE}} ame-minio -o jsonpath='{.status.loadBalancer.ingress[0].ip}'):9000
  export S3_ACCESS_ID=minio
  export S3_SECRET=minio123
  cargo run --bin ame-server

build_server_image:
  docker build . -f Dockerfile.server -t {{SERVER_IMAGE_TAG}}

push_server_image: 
  docker push {{SERVER_IMAGE_TAG}}

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

deploy_minio:
  #!/bin/sh
  cd ./manifests/minio/base/resources
  kustomize edit set namespace {{TARGET_NAMESPACE}}
  kustomize build . | kubectl apply -f -
  sleep 2
  kubectl wait pods -n {{TARGET_NAMESPACE}} -l app=ame-minio --for condition=Ready --timeout 90s

build_and_deploy_server: build_server_image push_server_image deploy_server

remove_server:
	kustomize build manifests/server/local | kubectl delete --ignore-not-found=true -f -
 
describe_server:
  kubectl describe pod -l app=ame-server -n {{TARGET_NAMESPACE}}

install_commit_template:
  git config commit.template ./.git_commit_message_template

start_opentelemtry_collector:
  docker run -d -p6831:6831/udp -p6832:6832/udp -p16686:16686 jaegertracing/all-in-one:latest

start_keycloak:
  docker run -p 8080:8080 -e KEYCLOAK_ADMIN=admin -e KEYCLOAK_ADMIN_PASSWORD=admin quay.io/keycloak/keycloak:20.0.1 start-dev

run_client:
  cargo run --bin ame-client

run_cli *ARGS:
 cargo run -p ame-cli -- {{ARGS}}

install_olm:
  operator-sdk olm install

install_keycloak_operator:
  kubectl apply -n {{TARGET_NAMESPACE}} -f https://raw.githubusercontent.com/keycloak/keycloak-k8s-resources/20.0.1/kubernetes/keycloaks.k8s.keycloak.org-v1.yml
  kubectl apply -n {{TARGET_NAMESPACE}} -f https://raw.githubusercontent.com/keycloak/keycloak-k8s-resources/20.0.1/kubernetes/keycloakrealmimports.k8s.keycloak.org-v1.yml
  kubectl apply -n {{TARGET_NAMESPACE}} -f https://raw.githubusercontent.com/keycloak/keycloak-k8s-resources/20.0.1/kubernetes/kubernetes.yml

add_helm_repos:
  helm repo add oauth2-proxy https://oauth2-proxy.github.io/manifests
  helm repo add ingress-nginx https://kubernetes.github.io/ingress-nginx
  helm repo add bitnami https://charts.bitnami.com/bitnami
  helm repo update

deploy_keycloak:
  helm install keycloak bitnami/keycloak --set auth.adminPassword=admin

deploy_oauth2_proxy:
  helm install oauth2-proxy oauth2-proxy/oauth2-proxy \
   --version 3.3.2 -f ./manifests/oauth2-proxy/oauth2proxy-values.yaml --wait

deploy_nginx:
  helm install ingress-nginx ingress-nginx/ingress-nginx \
  --wait --version 3.34.0 --set-string controller.config.ssl-redirect=false

test_cli:
  cargo test -p ame-cli

test_server:
  cargo test -p service

server_logs *ARGS:
  kubectl logs -n {{TARGET_NAMESPACE}} -l app=ame-server {{ARGS}}
