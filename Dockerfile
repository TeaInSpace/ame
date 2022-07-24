# Build the manager binary
FROM golang:1.18 as builder

WORKDIR /workspace
# Copy the Go Modules manifests
COPY go.mod go.mod
COPY go.sum go.sum
# cache deps before building and copying source so that we don't need to re-download as much
# and so that source changes don't invalidate our downloaded layer
RUN go mod download

# Copy the go source
COPY main.go main.go
COPY api/ api/
COPY controllers/ controllers/
COPY generated/ generated/
COPY server/ server/


# Builder the ame controller.
######################################################
from builder as task-controller-builder
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -a -o manager main.go

# Package the task controller in a minimal base image.
# Refer to https://github.com/GoogleContainerTools/distroless for more details
#####################################################
FROM gcr.io/distroless/static:nonroot as task-controller
WORKDIR /
COPY --from=task-controller-builder /workspace/manager .
USER 65532:65532

ENTRYPOINT ["/manager"]

# Build the ame-server
#########################################################
FROM builder as ame-server-builder
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -a -o ame_server teainspace.com/ame/server
RUN GRPC_HEALTH_PROBE_VERSION=v0.3.1 && \
    wget -qO/bin/grpc_health_probe https://github.com/grpc-ecosystem/grpc-health-probe/releases/download/${GRPC_HEALTH_PROBE_VERSION}/grpc_health_probe-linux-amd64 && \
    chmod +x /bin/grpc_health_probe

# Package the ame server in a minimal base image.
# Refer to https://github.com/GoogleContainerTools/distroless for more details
# ######################################################
from gcr.io/distroless/static:nonroot as ame-server
WORKDIR /
COPY --from=ame-server-builder /workspace/ame_server /bin
COPY --from=ame-server-builder /bin/grpc_health_probe /bin
user 65532:65532
ENTRYPOINT ["/bin/ame_server"]
