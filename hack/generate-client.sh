#!/bin/sh

# API_DIR is the current directory for defining the types for the AME CRDs.
API_DIR="api/v1alpha1"

# This is the directory client-gen tool will use to look for go types and output the generated client.
PACKAGE_DIR="pkg/apis/ame/v1alpha1"

OUTPUT_DIR=./generated

mkdir -p $PACKAGE_DIR

# The types from API_DIR are copied to PACKAGE_DIR so they are available for the client-gen tool.
cp -r $API_DIR/* $PACKAGE_DIR

# We use the generate-groups script to call the client-gen tool as it is provided from the official
# code-generator repository https://github.com/kubernetes/code-generator to interact with the code-generation tools.
# Note that the generate-groups.sh script has been slightly modified to work in this project.
./hack/generate-groups.sh client $OUTPUT_DIR ../ame/pkg/apis/ "ame:v1alpha1"

# The following sed invocations replace import path errors caused by the fact that the client-gen tool expects a different
# directory structure to the one used in this project.
sed -i -e 's@\.\./ame/pkg/apis/ame/@teainspace\.com/ame/api/@g' generated/clientset/versioned/typed/ame/v1alpha1/fake/fake_task.go
sed -i -e 's@\.\./ame/pkg/apis/ame/@teainspace\.com/ame/api/@g' generated/clientset/versioned/typed/ame/v1alpha1/fake/fake_reccurringtask.go
sed -i -e 's@\.\./ame/pkg/apis/ame/@teainspace\.com/ame/api/@g' generated/clientset/versioned/fake/register.go
sed -i -e 's@\.\./ame/pkg/apis/ame/@teainspace\.com/ame/api/@g' generated/clientset/versioned/scheme/register.go
sed -i -e 's@\.\./ame/pkg/apis/ame/@teainspace\.com/ame/api/@g' generated/clientset/versioned/typed/ame/v1alpha1/ame_client.go
sed -i -e 's@\.\./ame/pkg/apis/ame/@teainspace\.com/ame/api/@g' generated/clientset/versioned/typed/ame/v1alpha1/reccurringtask.go
sed -i -e 's@\.\./ame/pkg/apis/ame/@teainspace\.com/ame/api/@g' generated/clientset/versioned/typed/ame/v1alpha1/task.go

sed -i -e 's@generated/clientset/versioned@teainspace.com/ame/generated/clientset/versioned@g' generated/clientset/versioned/typed/ame/v1alpha1/fake/fake_task.go
sed -i -e 's@generated/clientset/versioned@teainspace.com/ame/generated/clientset/versioned@g' generated/clientset/versioned/typed/ame/v1alpha1/fake/fake_reccurringtask.go
sed -i -e 's@generated/clientset/versioned@teainspace.com/ame/generated/clientset/versioned@g' generated/clientset/versioned/typed/ame/v1alpha1/fake/fake_ame_client.go
sed -i -e 's@generated/clientset/versioned@teainspace.com/ame/generated/clientset/versioned@g' generated/clientset/versioned/fake/clientset_generated.go
sed -i -e 's@generated/clientset/versioned@teainspace.com/ame/generated/clientset/versioned@g' generated/clientset/versioned/clientset.go
sed -i -e 's@generated/clientset/versioned@teainspace.com/ame/generated/clientset/versioned@g' generated/clientset/versioned/typed/ame/v1alpha1/ame_client.go
sed -i -e 's@generated/clientset/versioned@teainspace.com/ame/generated/clientset/versioned@g' generated/clientset/versioned/typed/ame/v1alpha1/task.go
sed -i -e 's@generated/clientset/versioned@teainspace.com/ame/generated/clientset/versioned@g' generated/clientset/versioned/typed/ame/v1alpha1/reccurringtask.go

# The following sed invocation corrects an error in the groupversionreource assumed by the client-gen tool. This is probably due 
# to the directory/project structure not being what client-gen expects.
sed -i -e 's@schema.GroupVersionResource{Group: "ame@schema.GroupVersionResource{Group: "ame.teainspace.com@g' generated/clientset/versioned/typed/ame/v1alpha1/fake/fake_task.go
sed -i -e 's@schema.GroupVersionResource{Group: "ame@schema.GroupVersionResource{Group: "ame.teainspace.com@g' generated/clientset/versioned/typed/ame/v1alpha1/fake/fake_reccurringtask.go
sed -i -e 's@schema.GroupVersionKind{Group: "ame@schema.GroupVersionKind{Group: "ame.teainspace.com@g' generated/clientset/versioned/typed/ame/v1alpha1/fake/fake_task.go
sed -i -e 's@schema.GroupVersionKind{Group: "ame@schema.GroupVersionKind{Group: "ame.teainspace.com@g' generated/clientset/versioned/typed/ame/v1alpha1/fake/fake_reccurringtask.go

# This removed the temporary pkg directory created to appease client-gen.
rm -r pkg
