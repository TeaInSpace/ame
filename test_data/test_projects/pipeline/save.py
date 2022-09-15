import sys
import time
import os

print("saving model")

mode = os.environ.get("MODE")
expected_mode = "save"
if mode != expected_mode:
    raise Exception(f"expected MODE env var to be {expected_mode}, but got {mode} instead")

time.sleep(2)
expected_data = "mymodel"
with open("./models/model.txt", "r", encoding="utf-8") as f:
    if f.read() != expected_data:
        raise Exception(f"expected to find model contents: {expected_data}, but found instead: {f.read()}" )
