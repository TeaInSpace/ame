
# Image URL to use all building/pushing image targets
IMG ?= ame-controller:local
SERVER_IMG ?= ame-server:local
# ENVTEST_K8S_VERSION refers to the version of kubebuilder assets to be downloaded by envtest binary.
ENVTEST_K8S_VERSION = 1.24.1

# Get the currently used golang install path (in GOPATH/bin, unless GOBIN is set)
ifeq (,$(shell go env GOBIN))
GOBIN=$(shell go env GOPATH)/bin
else
GOBIN=$(shell go env GOBIN)
endif

# Setting SHELL to bash allows bash commands to be executed by recipes.
# This is a requirement for 'setup-envtest.sh' in the test target.
# Options are set to exit when a recipe line exits non-zero or a piped command fails.
SHELL = /usr/bin/env bash -o pipefail
.SHELLFLAGS = -ec

.PHONY: all
all: build

##@ General

# The help target prints out all targets with their descriptions organized
# beneath their categories. The categories are represented by '##@' and the
# target descriptions by '##'. The awk commands is responsible for reading the
# entire set of makefiles included in this invocation, looking for lines of the
# file as xyz: ## something, and then pretty-format the target and help. Then,
# if there's a line with ##@ something, that gets pretty-printed as a category.
# More info on the usage of ANSI control characters for terminal formatting:
# https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_parameters
# More info on the awk command:
# http://linuxcommand.org/lc3_adv_awk.php


# TODO: Clean up the depency/tool installation throughout the makefile.

.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Development

.PHONY: manifests
manifests: generate ## Generate WebhookConfiguration, ClusterRole and CustomResourceDefinition objects.
	$(CONTROLLER_GEN) rbac:roleName=manager-role crd webhook paths="./..." output:crd:artifacts:config=config/crd/bases

.PHONY: generate
generate: goimports gogo-protobuf go-to-protobuf client-gen controller-gen  ## Generate code containing DeepCopy, DeepCopyInto, and DeepCopyObject method implementations.
	./hack/generate-protobuf.sh
	./hack/generate-client.sh
	$(CONTROLLER_GEN) object:headerFile="hack/boilerplate.go.txt" paths="./..."

.PHONY: fmt
fmt: ## Run go fmt against code.
	go fmt ./...

.PHONY: vet
vet: ## Run go vet against code.
	go vet ./...

generate_test_env:
	./hack/generate_test_env.sh

.PHONY: test
test: vet fmt envtest generate_test_env # Run tests.
	# Note that the argument "-p 1" ensures that tests for multiple packages are not run asynchronously.
	# This is important as tests are operating on the same cluster and can therefore interfere with each other.
	KUBEBUILDER_ASSETS="$(shell $(ENVTEST) use $(ENVTEST_K8S_VERSION) -p path)" go test ./... -coverprofile cover.out -p 1 -timeout 15m

##@ Build

.PHONY: build
build: generate fmt vet ## Build manager binary.
	go build -o bin/manager main.go

.PHONY: run
run: fmt vet ## Run a controller from your host.
	go run ./main.go

.PHONY: docker-build
docker-build: test ## Build docker image with the manager.
	docker build -t ${IMG} .

.PHONY: docker-push
docker-push: ## Push docker image with the manager.
	docker push ${IMG}

docker-build-server: ## Builder docker image for the server.
	docker buildx build . --target ame-server -t ${IMG}

##@ Deployment

ifndef ignore-not-found
  ignore-not-found = true
endif

.PHONY: install
install: ## Install CRDs into the K8s cluster specified in ~/.kube/config.
	$(KUSTOMIZE) build config/crd | kubectl apply -f -

.PHONY: uninstall
uninstall: kustomize ## Uninstall CRDs from the K8s cluster specified in ~/.kube/config. Call with ignore-not-found=true to ignore resource not found errors during deletion.
	$(KUSTOMIZE) build config/crd | kubectl delete --ignore-not-found=$(ignore-not-found) -f -

.PHONY: deploy
deploy: kustomize ## Deploy controller to the K8s cluster specified in ~/.kube/config.
	cd config/manager && $(KUSTOMIZE) edit set image controller=${IMG}
	cd config/server && $(KUSTOMIZE) edit set image ame-server=${SERVER_IMG}
	$(KUSTOMIZE) build config/default | kubectl apply -f -
	sleep 1
	kubectl wait pods -n ame-system -l app=ame-server --for condition=Ready --timeout=90s
	kubectl wait pods -n ame-system -l control-plane=controller-manager --for condition=Ready --timeout=90s


.PHONY: undeploy
undeploy: ## Undeploy controller from the K8s cluster specified in ~/.kube/config. Call with ignore-not-found=true to ignore resource not found errors during deletion.
	$(KUSTOMIZE) build config/default | kubectl delete --ignore-not-found=$(ignore-not-found) -f -

CONTROLLER_GEN = $(shell pwd)/bin/controller-gen
.PHONY: controller-gen
controller-gen: ## Download controller-gen locally if necessary.
	GOBIN=$(PROJECT_DIR)/bin go install sigs.k8s.io/controller-tools/cmd/controller-gen@v0.9.0

KUSTOMIZE = $(shell pwd)/bin/kustomize
.PHONY: kustomize
kustomize: ## Download kustomize locally if necessary.
	GOBIN=$(PROJECT_DIR)/bin go install sigs.k8s.io/kustomize/kustomize/v4@v4.5.5 

	

ENVTEST = $(shell pwd)/bin/setup-envtest
.PHONY: envtest
envtest: ## Download envtest-setup locally if necessary.
	GOBIN=$(PROJECT_DIR)/bin go install sigs.k8s.io/controller-runtime/tools/setup-envtest@latest

# go-get-tool will 'go get' any package $2 and install it to $1.
PROJECT_DIR := $(shell dirname $(abspath $(lastword $(MAKEFILE_LIST))))

enable-commit-message-template:
	git config commit.template .git_commit_message_template

client-gen:
	go install k8s.io/code-generator/cmd/client-gen@v0.24.1

go-to-protobuf:
	GOBIN=$(PROJECT_DIR)/bin go install k8s.io/code-generator/cmd/go-to-protobuf@v0.21.5

gogo-protobuf:
	go install github.com/gogo/protobuf/proto
	go install github.com/gogo/protobuf/jsonpb
	go install github.com/gogo/protobuf/protoc-gen-gogo
	go install github.com/gogo/protobuf/gogoproto
	go install github.com/gogo/protobuf/protoc-gen-gofast

goimports:
	go install golang.org/x/tools/cmd/goimports@latest

kind:
	go install sigs.k8s.io/kind@v0.14.0 

tools: kind goimports gogo-protobuf go-to-protobuf client-gen kustomize

deploy_local_cluster: manifests create_local_cluster prepare_local_cluster load_local_images install deploy

create_local_cluster:
	kind create cluster

prepare_local_cluster:
	kubectl apply -f https://raw.githubusercontent.com/metallb/metallb/v0.12.1/manifests/namespace.yaml
	kubectl apply -f https://raw.githubusercontent.com/metallb/metallb/v0.12.1/manifests/metallb.yaml
	sleep 20
	kubectl wait pods -n metallb-system -l app=metallb --for condition=Ready --timeout=90s
	kubectl apply -f ./metallb_config.yaml
	kubectl create ns ame-system --dry-run=client -o yaml | kubectl apply -f -
	kubectl apply -n ame-system -f https://raw.githubusercontent.com/argoproj/argo-workflows/master/manifests/quick-start-postgres.yaml
	kubectl apply -n ame-system -f ./config/minio/
	kubectl apply -n ame-system -f ./config/argo/ame_executor_template.yaml


load_local_images: load_executor
	docker buildx build . --target ame-server -t ame-server:local
	docker buildx build . --target task-controller -t ame-controller:local
	kind load docker-image ame-server:local
	kind load docker-image ame-controller:local
	kubectl delete pod -l app=ame-server -n ame-system
	kubectl delete pod -l control-plane=controller-manager -n ame-system

load_executor:
	docker build ./executor/ -t ame-executor:local
	kind load docker-image ame-executor:local

update_wf_template:
	kubectl apply -n ame-system -f ./config/argo/ame_executor_template.yaml


refresh_deployment: undeploy prepare_local_cluster  load_local_images deploy

delete_local_cluster:
	kind delete cluster
