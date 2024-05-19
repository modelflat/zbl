import argparse
import zbl

parser = argparse.ArgumentParser("zbl")
parser.add_argument("--window-name", type=str, required=False, default=None)
parser.add_argument("--display-id", type=int, required=False, default=None)
parser.add_argument("--is-cursor-capture-enabled", action="store_true", required=False)
parser.add_argument("--is-border-required", action="store_true", required=False)

zbl.show(parser.parse_args())
