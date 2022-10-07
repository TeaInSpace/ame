#!/bin/bash

# kubectl create secret -n ame-system docker-registry regcred --docker-server='ghcr.io' --docker-username='jmintb' --docker-password='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' 
# exit 0

set -e

CLUSTER_NAME="p100"
REGION="us-west4-a"

gcloud container clusters create p100   --accelerator type=nvidia-tesla-t4,count=1   --zone us-west4-a --machine-type n1-standard-4 --num-nodes 1


#gcloud container clusters update my-gke-cluster  --enable-autoprovisioning  --min-accelerator type=nvidia-tesla-k80,count=1     --max-accelerator type=nvidia-tesla-t4,count=4 --max-cpu 200 --max-memory 500 --zone $REGION
gcloud container clusters get-credentials --region $REGION $CLUSTER_NAME
#gcloud container clusters update $CLUSTER_NAME --enable-autoprovisioning \
# --autoprovisioning-scopes=https://www.googleapis.com/auth/logging.write,https://www.googleapis.com/auth/monitoring,https://www.googleapis.com/auth/devstorage.read_only,https://www.googleapis.com/auth/compute --zone $REGION

set +e

kubectl create ns ame-system
kubectl create clusterrolebinding jessie-cluster-admin-binding --clusterrole=cluster-admin --user=polki4459@gmail.com

# Add container registry secret
kubectl create secret -n ame-system docker-registry regcred --docker-server='ghcr.io' --docker-username='jmintb' --docker-password='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' 

kubectl apply -f https://raw.githubusercontent.com/GoogleCloudPlatform/container-engine-accelerators/master/nvidia-driver-installer/ubuntu/daemonset-preloaded.yaml
