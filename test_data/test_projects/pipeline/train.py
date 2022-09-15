import sys
import time
import os

print("training model")

mode = os.environ.get("MODE")
expected_mode = "training"
if mode != expected_mode:
    raise Exception(f"expected MODE env var to be {expected_mode}, but got {mode} instead")

expected_data = "mydata"
with open("./data/data.txt", "r", encoding="utf-8") as f:
    if f.read() != expected_data:
        raise Exception(f"expected to find data: {expected_data}, but got {f.read()} instead")

with open("./models/model.txt", "w", encoding="utf-8") as f:
    f.write("mymodel")
