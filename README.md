# zbl

[![PyPI version](https://badge.fury.io/py/zbl.svg)](https://badge.fury.io/py/zbl)

`zbl` is a Rust and Python library for screen/window capturing on Windows. It provides an interface
to `Windows.Graphics.Capture` API with a focus on simplifying integrating computer vision applications for Windows Desktop apps.

**This library is in early development stage**. This means that it's only verified to work for a 'happy path'
scenarios - beware of bugs!

## Python

### Installation

`pip install zbl`

Alternatively, you can install suitable wheel from [releases page](https://github.com/modelflat/zbl/releases).

### Usage

```python
from zbl import Capture

with Capture(window_name='visual studio code') as cap:
    # grab a single frame (numpy.ndarray) and print its shape
    print(cap.grab().shape)
```

The snippet above will capture a window which title contains the string `visual studio code`,
grab one frame and print its shape.

See [Capture](https://github.com/modelflat/zbl/blob/master/zbl_py/zbl/__init__.py) for other API methods.

To run an example using OpenCV's `highgui`:

1. Install `opencv-python`
2. Run `python -m zbl --display-id 0`

## Rust

See [examples](https://github.com/modelflat/zbl/tree/master/zbl/examples).
Note: if you are getting OpenCV build errors when building the example, check out [how to build OpenCV rust bindings](https://github.com/twistedfall/opencv-rust#rust-opencv-bindings).

## Comparison to `mss` / `pyautogui`

Both are very slow at the time of writing. `mss` tops at 30-50 fps in a tight loop, `pyautogui` is
even slower than that. `zbl` is able to capture an order of magnitude faster (at 500-700 fps). This allows a lot more time for the actual processing.

## Credits

`zbl` is heavily inspired by [screenshot-rs](https://github.com/robmikh/screenshot-rs).
