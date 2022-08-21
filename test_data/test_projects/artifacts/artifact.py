import sys

with open('./generated/myartifact.txt', 'w', encoding='utf-8') as f:
    for a in sys.argv[1:]:
        f.write(a)

