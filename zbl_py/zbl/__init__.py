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

    def grab(self) -> numpy.ndarray:
        """
        Same as `grab_raw`, but converts captured frame to numpy array.
        """
        return frame_to_numpy_array(self.grab_raw())

    def grab_raw(self) -> Frame:
        """
        Grab the next frame from the capture.

        Blocks until the frame is available.
        Raises `StopIteration` when no more frames can be received (e.g. capture closed).
        """
        return self._inner.grab()

    def try_grab(self) -> Optional[numpy.ndarray]:
        """
        Same as `try_grab_raw`, but converts captured frame to numpy array.
        """
        frame = self.try_grab_raw()
        if frame is None:
            return None
        return frame_to_numpy_array(frame)

    def try_grab_raw(self) -> Optional[Frame]:
        """
        Try grabbing the next frame from the capture.

        Returns `None` if the frame is not ready yet.
        Raises `StopIteration` when no more frames can be received (e.g. capture closed).
        """
        return self._inner.try_grab()

    def frames(self) -> Iterator[numpy.ndarray]:
        """
        Returns an iterator over numpy frames in this capture. 
        """
        for frame in self.frames_raw():
            yield frame_to_numpy_array(frame)

    def frames_raw(self) -> Iterator[Frame]:
        """
        Returns an iterator over frames in this capture.
        """
        while True:
            try:
                yield self.grab_raw()
            except StopIteration as _:
                return

    def raw_frames(self) -> Iterator[Frame]:
        """
        Deprecated, prefer `frames_raw` instead.
        """
        yield from self.frames_raw()

    def __enter__(self) -> "Capture":
        self._inner.start()
        return self

    def __exit__(self, *_):
        self._inner.stop()
