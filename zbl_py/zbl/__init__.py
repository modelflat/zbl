import numpy
import ctypes as C

from typing import Iterator, Optional


from .zbl import Capture as _NativeCapture, Frame


uint8_ptr = C.POINTER(C.c_uint8)


def frame_to_numpy_array(frame: Frame) -> numpy.ndarray:
    arr = numpy.ctypeslib.as_array(
        C.cast(frame.ptr, uint8_ptr), shape=(frame.height, frame.row_pitch))
    if frame.row_pitch == frame.width * 4:
        return arr.reshape(frame.height, frame.width, 4)
    else:
        # TODO copy to avoid slow access?
        return arr[:, :frame.width * 4].reshape((frame.height, frame.width, 4))


class Capture:

    def __init__(
        self,
        window_name: Optional[str] = None,
        window_handle: Optional[str] = None,
        display_id: Optional[int] = None,
        capture_cursor: bool = False,
    ):
        self._inner = _NativeCapture(window_name, window_handle, display_id, capture_cursor)

    @property
    def handle(self) -> int:
        return self._inner.handle()

    def raw_frames(self) -> Iterator[Frame]:
        while True:
            next_frame = self._inner.grab()
            if next_frame is None:
                break
            yield next_frame

    def frames(self) -> Iterator[numpy.ndarray]:
        for frame in self.raw_frames():
            yield frame_to_numpy_array(frame)

    def __enter__(self) -> 'Capture':
        self._inner.start()
        return self

    def __exit__(self, *_):
        self._inner.stop()


def show(args):
    from time import perf_counter
    import cv2

    try:
        cv2.namedWindow('zbl', cv2.WINDOW_NORMAL)

        with Capture(
            window_name=args.window_name,
            display_id=args.display_id,
            capture_cursor=args.capture_cursor
        ) as cap:
            t = perf_counter()
            last_print = perf_counter()
            t_total = 0
            frames = 0
            for frame in cap.frames():
                t = perf_counter() - t
                t_total += t
                frames += 1
                if perf_counter() - last_print > 1:
                    print(f'[zbl] capture fps: {frames / t_total:.3f}')
                    t_total = 0
                    frames = 0
                    last_print = perf_counter()
                cv2.imshow('zbl', frame)
                if cv2.waitKey(8) != -1:
                    break
                t = perf_counter()

        cv2.destroyAllWindows()
    except KeyboardInterrupt:
        pass
