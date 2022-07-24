#!/bin/sh

# This script wraps the go-to-protobuf tool to make it usable with the current directory structure.
# It seems to have strict and undocumented requirements as to how the directory is structured for it
# to detect a package to generate protobuf files for.

# API_DIR is the current directory for defining the types for the AME CRDs.
API_DIR="api/v1alpha1"

# This is the directory go-to-protobuf will use to look for go types and output the associated 
# protobuf files and generated go code from those files. The direcory structure is very specific
# and can't be changes as of writing this.
PACKAGE_DIR="pkg/apis/ame/v1alpha1"

# We need to export the GOPATH env variable so the go-to-protobuf tool knows where
# to place the generated files both for k8s protobuf types and the protobuf types
# for this project.
export GOPATH=$(go env GOPATH)

# The types are then copied from API_DIR to PACKAGE_DIR.
mkdir -p $PACKAGE_DIR
cp -r ./api/v1alpha1/* $PACKAGE_DIR

# groupversion_info.go is removed as it contains no types needed to generate protobuf files and it 
# causes issues with go-to-protobuf.
rm $PACKAGE_DIR/groupversion_info.go 

# We clone the gogo/protobuf repository inorder to get the protobuf definitions required for using
# gogo/protobuf tools. The repo is cloned to $GOPATH/src inorder to place it at the same location as the
# other protobuf definitions generated and used by the go-to-protobuf tool.
git clone --depth 1 https://github.com/gogo/protobuf.git -b v1.3.2 $GOPATH/src/github.com/gogo/protobuf

# Note that the header file is empty but go-to-protobuf complains if no file is supplied.
# For the packages argument we are required to prepend "teainspace.com/ame" the reason is not
# documented but it does match the go module import structure to include the module name
# before referencing a package/import. Either way go-to-buf will not work with a direct path
# reference.
# The proto-import argument is used to import proto files from dependencies in the go path,
# this is required as the generated protobuf files are importing protobuf definitions from 
# various k8s libraries. go-to-protobuf will place the k8s protobuf definitions at GOPATH/src
# therefore this is also the location we supply to the proto-import argument.
./bin/go-to-protobuf \
  --go-header-file=./hack/boilerplate.go.txt \
  --packages="teainspace.com/ame/"$PACKAGE_DIR \
  --proto-import=$(go env GOPATH)/src \

# The generated files are moved to the original location of the CRD types as this is where they will
# be imported from.
mv $GOPATH/src/"teainspace.com/ame/"$PACKAGE_DIR/*.proto $API_DIR/ 
mv $GOPATH/src/"teainspace.com/ame/"$PACKAGE_DIR/*.pb.go $API_DIR/ 

# We make sure to cleanup the temporary package directory we created to appease go-to-protobuf.
rm -r pkg

# Generate a gRPC server using the protobuf definitions for the api types.
# Here we again import from GOPATH/src as this is where all of the protobuf definitions were placed by 
# go-to-protobuf.
echo "Generating gRPC server"
protoc -I=. -I=$GOPATH/src --gofast_out=plugins=grpc:. ./server/cmd/task.proto
protoc -I=. -I=$GOPATH/src --gofast_out=plugins=grpc:. ./server/cmd/health.proto

# Correct import path errors caused by the directory structure of this project differing from what go-to-protobuf expects.
sed -i -e 's/api\/v1alpha1/teainspace.com\/ame\/api\/v1alpha1/g' server/cmd/task.pb.go
sed -i -e 's/grpc_health_v1/task/g' server/cmd/health.pb.go
