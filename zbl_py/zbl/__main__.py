import argparse

from time import perf_counter

import cv2
import zbl


def show(args):
    try:
        cv2.namedWindow("zbl", cv2.WINDOW_NORMAL | cv2.WINDOW_KEEPRATIO)

        with zbl.Capture(
            window_name=args.window_name,
            display_id=args.display_id,
            is_cursor_capture_enabled=args.is_cursor_capture_enabled,
            is_border_required=args.is_border_required,
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
                    print(f"[zbl] capture fps: {frames / t_total:.1f}")
                    t_total = 0
                    frames = 0
                    last_print = perf_counter()
                cv2.imshow("zbl", frame)
                if cv2.waitKey(8) != -1:
                    break
                t = perf_counter()

        cv2.destroyAllWindows()
    except KeyboardInterrupt:
        pass


def main():
    parser = argparse.ArgumentParser("zbl")
    parser.add_argument("--window-name", type=str, required=False, default=None)
    parser.add_argument("--display-id", type=int, required=False, default=None)
    parser.add_argument(
        "--is-cursor-capture-enabled", action="store_true", required=False
    )
    parser.add_argument("--is-border-required", action="store_true", required=False)

    show(parser.parse_args())


main()
