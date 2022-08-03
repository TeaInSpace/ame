## Walkthrough

Here we will go through a typical cycle of using AME from setting up a new project, to having an automated machine learning pipeline.

### CLI setup

#### Installation

TODO

#### Authentication

The administrator of your AME instance will provide you with a bearer token for authentication. Once you have that authenticate as shown below:

TODO: gif

#### Creating the first task

This can look very differently depending on your project, to keeps things brief we will focus on a single Task workflow, where you run your training using a python file train.py.

TODO: gif

#### Running the first task

After creating your first task you will probably want to run it. Note when running it how any artifacts are transferred back to your local  machine and how logs are displayed just as if you were running the task locally. AME attempts to create an almost transparent remote execution experience for when you are experimenting with your project from your machine.

TODO: gif

#### Creating a pipeline

Inorder to conserve compute resources and allow for cacheing you might want to split up your task into multiple sub tasks. AME supports pipelines of Tasks with individual requirenments. All of the project context is carried over between steps in the pipeline so if your first task saves files to the project directory, they will be available in the next step.

TODO: gif

#### Scheduling and git tracking

Once you have a pipeline setup you can schedule it for recurring execution using a RecurringTask and including ame.yaml in your version control.

TODO: gif, file example
