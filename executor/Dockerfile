FROM ubuntu:22.04

# These packages are required by pyenv, see https://github.com/pyenv/pyenv/wiki#suggested-build-environment 
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install\
    git make build-essential libssl-dev zlib1g-dev \
    libbz2-dev libreadline-dev libsqlite3-dev wget \
    curl llvm libncursesw5-dev xz-utils tk-dev \
    libxml2-dev libxmlsec1-dev libffi-dev liblzma-dev -y

# The execute script is called by the argo workflow to execute the command supplied by the Task.
COPY ./execute.sh  /usr/bin/execute
COPY ./save_artifacts.sh  /usr/bin/save_artifacts
RUN chmod +x /usr/bin/execute
RUN chmod +x /usr/bin/save_artifacts

# It is important that AME does not execute tasks as root as AME is executing unknown code which
# could be malicious.
RUN adduser -u 1001 --disabled-password ame
# Tasks are executed within the /project directory, therefore the ame user should require access
# this directory.
RUN mkdir /project
RUN chown ame -R /project
USER ame

# THe ame user gets its home directory where pyenv and pipenv will end up installed
# therefore the .local/bin needs to be included in the PATH environment variable.
ENV PATH="/home/ame/.pyenv/shims:/home/ame/.pyenv/bin:/home/ame/.local/bin:${PATH}"

# Pipenv uses pyenv to install new python versions, the installation process is 
# ducumented here: https://github.com/pyenv/pyenv#installation
# TODO: We need a way to validate that pyenv is installed correctly, from the official repo. 
RUN PYENV_GIT_TAG=v2.3.4 curl https://pyenv.run | bash
RUN echo 'export PYENV_ROOT="$HOME/.pyenv"' >> ~/.bashrc
RUN echo 'export PATH="$PYENV_ROOT/bin:$PATH"' >> ~/.bashrc
RUN echo 'eval "$(pyenv init --path)"' >> ~/.bashrc
RUN pyenv install 3.11
RUN pyenv global 3.11
RUN pyenv rehash
RUN python --version

# Install pipenv
RUN pip install --upgrade pip
RUN pip install pipenv

# Setup the s3cmd used for interacting with the cluster's object storage during
# Task execution.
RUN pip install s3cmd

RUN pip install mlflow[extras]

# TODO: pin poetry versionk
RUN curl -sSL https://install.python-poetry.org | python -
 
WORKDIR /project
