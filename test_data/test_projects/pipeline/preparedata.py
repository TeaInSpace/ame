import time
import sys
import os

mode = os.environ.get("MODE")
if mode != "dataprep":
    raise Exception(f"expected MODE env var to be dataprep, but got {mode} instead")

s3_secret = os.environ.get("STORAGE_S3_SECRET")
expected_s3_secret = "sometoken"
if expected_s3_secret != s3_secret:
    raise Exception(f"expected STORAGE_S3_SECRET to be sometoken, but got {s3_secret} instead")


s3_bucket = os.environ.get("S3_BUCKET")
expected_s3_bucket = "mybucket"
if s3_bucket != expected_s3_bucket:
    raise Exception(f"expected S3_BUCKET env var to be {expected_s3_bucket} but got {s3_bucket} instead")

print("preparing data")
time.sleep(2)

with open("./data/data.txt","w",encoding="utf-8"  ) as f:
    f.write("mydata")

