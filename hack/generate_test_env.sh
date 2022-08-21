#!/bin/sh

AME_SERVER_ENDPOINT=$(kubectl get services -n ame-system  ame-ame-server-service --output jsonpath='{.status.loadBalancer.ingress[0].ip}'):3342
AME_OBJECT_STORAGE_ENDPOINT=http://$(kubectl get services -n ame-system  ame-minio --output jsonpath='{.status.loadBalancer.ingress[0].ip}'):9000
AME_AUTH_TOKEN=mytoken
AME_BUCKET=ameprojectstorage
AME_NAMESPACE=ame-system

FILE_NAME=test.env
# This ensures an empty cli.yaml file.
:> $FILE_NAME

echo AME_SERVER_ENDPOINT=$AME_SERVER_ENDPOINT >> $FILE_NAME
echo AME_OBJECT_STORAGE_ENDPOINT=$AME_OBJECT_STORAGE_ENDPOINT >> $FILE_NAME
echo AME_AUTH_TOKEN=$AME_AUTH_TOKEN >> $FILE_NAME
echo AME_NAMESPACE=$AME_NAMESPACE >> $FILE_NAME
echo AME_BUCKET=$AME_BUCKET >> $FILE_NAME
