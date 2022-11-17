# zbl

[![PyPI version](https://badge.fury.io/py/zbl.svg)](https://badge.fury.io/py/zbl)

`zbl` is a Rust and Python library which aims to make it easy to integrate CV libraries (such as OpenCV) with
Windows Desktop apps for real-time processing. It does so by providing a simplified interface to 
`Windows.Graphics.Capture`.

**This library is not well-tested against corner cases, and was only verified to work for a 'happy path' scenarios, so beware of bugs!**

## Python

### Installation

`pip install zbl`

Alternatively, you can install suitable wheel from [releases page](https://github.com/modelflat/zbl/releases).

### Usage

```python
from zbl import Capture

with Capture('visual studio code') as cap:
    frame = next(cap.frames())
    print(frame.shape)
```

The snippet above will capture a window which title contains the string `visual studio code`, take one frame (which is represented as a `numpy` array) and print its shape.

To run an example using OpenCV's `highgui`:

1. Install `opencv-python`
2. Run `python -m zbl '<full or partial window name, case insensitive>'`

## Rust

See [examples](https://github.com/modelflat/zbl/tree/master/zbl/examples).
Note: if you are getting OpenCV build errors when building the example, check out [how to build OpenCV rust bindings](https://github.com/twistedfall/opencv-rust#rust-opencv-bindings).

## Why not `mss` / `pyautogui`?

Those are the definition of "slow" at the time of writing. `mss` tops at 30-50 fps in a tight loop, `pyautogui` is
even slower than that. Due to GPU accel which comes with D3D11 and pointer sharing, `zbl` captures at 500-700 fps - an order of magnitude faster, which allows a lot more time for the actual processing.

## Credits

`zbl` is heavily inspired by [screenshot-rs](https://github.com/robmikh/screenshot-rs).
