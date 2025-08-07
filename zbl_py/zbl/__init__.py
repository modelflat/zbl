import numpy
import ctypes as C

from typing import Iterator, Optional


from .zbl import Capture as _NativeCapture, Frame


uint8_ptr = C.POINTER(C.c_uint8)

# set process to be DPI-aware
_ = C.windll.shcore.SetProcessDpiAwareness(2)


def frame_to_numpy_array(frame: Frame) -> numpy.ndarray:
    arr = numpy.ctypeslib.as_array(
        C.cast(frame.ptr, uint8_ptr), shape=(frame.height, frame.row_pitch)
    )
    if frame.row_pitch == frame.width * 4:
        return arr.reshape(frame.height, frame.width, 4)
    else:
        return arr[:, : frame.width * 4].reshape((frame.height, frame.width, 4))


class Capture:
    def __init__(
        self,
        window_name: Optional[str] = None,
        window_handle: Optional[str] = None,
        display_id: Optional[int] = None,
        is_cursor_capture_enabled: bool = False,
        is_border_required: bool = True,
        use_staging_texture: bool = True,
    ):
        self._inner = _NativeCapture(
            window_name,
            window_handle,
            display_id,
            is_cursor_capture_enabled,
            is_border_required,
            use_staging_texture,
        )

    @property
    def handle(self) -> int:
        return self._inner.handle()

    def grab(self) -> Frame:
        """
        Grab the next frame from the capture.

        Blocks until the frame is available.
        Raises `StopIteration` when no more frames can be received (e.g. capture closed).
        """
        return self._inner.grab()

    def try_grab(self) -> Optional[Frame]:
        """
        Try grabbing the next frame from the capture.

        Returns `None` if the frame is not ready yet.
        Raises `StopIteration` when no more frames can be received (e.g. capture closed).
        """
        return self._inner.try_grab()

    def raw_frames(self) -> Iterator[Frame]:
        while True:
            try:
                yield self.grab()
            except StopIteration:
                break

    def frames(self) -> Iterator[numpy.ndarray]:
        for frame in self.raw_frames():
            yield frame_to_numpy_array(frame)

    def __enter__(self) -> "Capture":
        self._inner.start()
        return self

    def __exit__(self, *_):
        self._inner.stop()
