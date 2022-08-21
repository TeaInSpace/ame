#!/bin/bash

# It is important that the script exits if something goes wrong,
# so the workflow can fail fast and AME can tak appropriate action.
set -e

PROJECT_COMMAND="$@"

pipenv sync

pipenv run $PROJECT_COMMAND
