# zbl

zbl is a Rust and Python library which provides a very simple interface for Microsoft's `Windows.Graphics.Capture` API
and makes it easy to integrate CV libraries (such as OpenCV) with desktop apps.

**This library is not well-tested against corner cases, and was only verified to work for a 'happy path' scenarios, so beware of bugs!**

## Installation

TODO

## Usage

### Python

```python
from zbl import Capture

with Capture('<window name>') as cap:
    frame = next(cap.frames())
    print(frame.shape)
```

To run an example using OpenCV's `highgui`:

1. Install `opencv-python`
2. Run `python -m zbl '<full or partial window name, case insensitive>'`

### Rust

See [examples](https://github.com/modelflat/zbl/tree/master/zbl/examples).
Note: if you are getting OpenCV build errors when building the example, check out [how to build OpenCV rust bindings](https://github.com/twistedfall/opencv-rust#rust-opencv-bindings).
