# Running a Task with the CLI

This example will introduce to [Tasks](Tasks) and the [CLI](cli). The complete project is also available in the examples [directory](directory).

If you followed the quick start your CLI should be connected to an AME instance. If in doubt run `ame check` and any issues will be reported.

## Your first Task

Before we can run a task we must have a project setup. To init a project follow the commands as shown below, replacing myproject with the 
path to your project.

```sh
mkdir myproject
cd myproject
ame init
```

Now you should have an AME file ame.yaml inside your project:
```yaml
name: myproject
```

Update your file to match the changes shown below.

```yaml
name: myproject
tasks:
  - name: hello
    !custom
    executor:
      pythonVersion: 3.11
      command: python hellow_world.py
```

and create a python file:
```python
# hello_world.py
print("Hello World")
```

To run this and see the output use `ame task run hello --logs`, this will execute the Task remotely and follow the logs in your
terminal. If you go the the AME dashboard you can see the Task and logs under ... TODO.

Note that we are using the [custom](custom) executor here as there are no dependencies for AME to install, otherwise we would
use an executor for the dependency manager used in our project.  Every Task must have an executor otherwise AME has no idea how
to execute it.
