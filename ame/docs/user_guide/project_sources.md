# Project sources

A project source informs AME of a location to check and sync an AME project from. Currently the only supported location is a Git repository.

## Git project sources

Git project sources allow for a Gitops like approach to managing models, data and the surrounding operations using the AME file defined in the repository.

### How to use Git project sources

You can create a Git project source either through the CLI or the AME frontend.

Below are a few examples with the CLI

```bash
# A public repository:
ame projectsrc create https://github.com/TeaInSpace/ame-demo.git

# A private repository:
ame projectsrc create https://github.com/TeaInSpace/ame-demo.git --secret MY_SECRET_ID

# Edit the secret for an existing project source:
ame projectsrc edit https://github.com/TeaInSpace/ame-demo.git --secret MY_SECRET_ID
```

AME will attempt to warn you of issues as early as possible. For example if AME fails to clone the the repository the CLI will make that clear.

Example:

TODO: insert image

Once AME has a valid project source it will check all branches for AME files and track them according to the tracking configuration specified in each file.
