CONTROLLER_NAME := "ame-controller"
SERVER_NAME := "ame-server"
EXECUTOR_NAME := "ame-executor"
ORG := "teainspace"
REGISTRY := "ghcr.io"

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

k3d:
  k3d cluster create main --servers 1 --registry-create main \
    --no-lb --no-rollback \
    --k3s-arg "--disable=traefik,servicelb,metrics-server@server:*" \
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

setup_cluster: k3d install_crd install_argo_workflows

start_controller:
  cargo run --bin controller


