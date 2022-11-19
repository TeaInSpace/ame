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
  k3d cluster create main --servers 1 --registry-create main \
    --no-lb --no-rollback \
    --k3s-arg "--disable=traefik,metrics-server@server:*" \
    --k3s-arg '--kubelet-arg=eviction-hard=imagefs.available<1%,nodefs.available<1%@agent:*' \
    --k3s-arg '--kubelet-arg=eviction-minimum-reclaim=imagefs.available=1%,nodefs.available=1%@agent:*'

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
 kubectl apply -f manifests/argo.yaml

setup_cluster: k3s install_crd install_argo_workflows
  kubectl create ns {{TARGET_NAMESPACE}}

start_controller:
  cargo run --bin controller

start_server:
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
  kubectl wait pods -n {{TARGET_NAMESPACE}} -l app=ame-server --for condition=Ready --timeout=90s

build_and_deploy_server: build_server_image push_server_image deploy_server

remove_server:
	kustomize build manifests/server/local | kubectl delete --ignore-not-found=true -f -
 
describe_server:
  kubectl describe pod -l app=ame-server -n {{TARGET_NAMESPACE}}

install_commit_template:
  git config commit.template ./.git_commit_message_template