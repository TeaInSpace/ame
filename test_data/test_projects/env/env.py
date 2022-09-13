import sys
import os

for a in sys.argv[1:]:
    (key, val) = a.split( "=")
    print("validating env: ", key,"=", val)

    envVal = os.environ.get(key)
    if envVal == "" or envVal is None:
        raise Exception(f"key: {key} was not found in environment")

    if envVal != val:
        raise Exception(f"key: {key} was expected to have value {val}, but got value {envVal} instead")
