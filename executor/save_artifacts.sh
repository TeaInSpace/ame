#!/bin/bash
# Note the following:

# - This script is meant to be executed by an Argo Workflow based on the template at
# ame/config/argo/ame_executor_template.yaml, the environment variables used in this script 
# are set in that file. 

# - The script expects to be executed from the within the directory of the project currently being 
# executed.

# This is the path in object storage where artifacts will be saved to.
ARTIFACT_STORAGE_PATH=$1

# The dry run flag is used to generate a list of files that are not present in the project files
# in object storage. This is a hack to avoid implementation file diffing for the prototype.
# grep is used to filter the output from the dryrun to get a list of file paths.
ARTIFACTS=$(s3cmd --no-ssl --region us-east-1 --host=$MINIO_URL --host-bucket=$MINIO_URL sync --dry-run ./ s3://$TASK_DIRECTORY/ | grep -oP \'.*?\' | grep -v s3 | grep -v git)

echo "Uploading artifacts: $ARTIFACTS"
          
# This checks that the artifacts list is not empty, before attempting to save them.
if [ ! -z "$ARTIFACTS" ] 
then

# The artifacts list has an extra set of quotes which is problematic for s3cmd, so they are removed
# here.
CLEANPATH=($(echo $ARTIFACTS| tr -d '\047'))

for path in "${CLEANPATH[@]}" 
do

# Filters out the first segment of the path as the relative path is required, to combine the the
# path in $ARTIFACTS_PATH variable, which stores the path to the artifacts for this task.
SUB_PATH=${path#*/}

s3cmd --no-ssl --region us-east-1 --host=$MINIO_URL --host-bucket=$MINIO_URL put $SUB_PATH s3://$ARTIFACT_STORAGE_PATH$SUB_PATH

done

fi
