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


from builder as task-controller-builder
# Build
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -a -o manager main.go

# Use distroless as minimal base image to package the manager binary
# Refer to https://github.com/GoogleContainerTools/distroless for more details
FROM gcr.io/distroless/static:nonroot as task-controller
WORKDIR /
COPY --from=builder /workspace/manager .
USER 65532:65532

ENTRYPOINT ["/manager"]

# Build the ame-server
#########################################################
FROM builder as ame-server-builder
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -a -o ame_server teainspace.com/ame/server

# Build the ame-server image 
# ######################################################
from ame-server-builder as ame-server
WORKDIR /
COPY --from=ame-server-builder /workspace/ame_server /bin
user 65532:65532
ENTRYPOINT ["/bin/ame_server"]
