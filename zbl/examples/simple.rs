use std::time::Instant;

use clap::Parser;
use opencv::{
    core::{Mat, Size, CV_8UC4},
    highgui,
};
use zbl::{Capturable, Capture, Display, Window};

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(long)]
    window_name: Option<String>,
    #[clap(long)]
    display_id: Option<usize>,
}

fn main() {
    zbl::init();

    let args = Args::parse();

    let target = if let Some(window_name) = args.window_name {
        let window = Window::find_first(&window_name).expect("failed to find window");
        Box::new(window) as Box<dyn Capturable>
    } else if let Some(display_id) = args.display_id {
        let display = Display::find_by_id(display_id).expect("failed to find display");
        Box::new(display) as Box<dyn Capturable>
    } else {
        panic!("either --window-name or --display-id should be set!");
    };

    let mut capture = Capture::new(target, true, true).expect("failed to initialize capture");

    capture.start().expect("failed to start capture");

    highgui::named_window("Test", highgui::WINDOW_NORMAL | highgui::WINDOW_KEEPRATIO)
        .expect("failed to setup opencv window");

    let mut total_time = 0f32;
    let mut total_seconds = 0;
    let mut total_frames = 0;
    let start = Instant::now();
    loop {
        let t_frame_start = Instant::now();
        if let Some(frame) = capture.grab().expect("failed to get frame") {
            let desc = frame.desc();
            let mat = unsafe {
                Mat::new_size_with_data(
                    Size::new(desc.Width as i32, desc.Height as i32),
                    CV_8UC4,
                    frame.mapped_ptr.pData,
                    frame.mapped_ptr.RowPitch as usize,
                )
            }
            .expect("failed to convert to opencv frame");
            let t_frame_end = Instant::now();

            highgui::imshow("Test", &mat).expect("failed to show frame");
            if highgui::wait_key(8).expect("failed to wait user input") != -1 {
                break;
            }

            total_frames += 1;
            total_time += (t_frame_end - t_frame_start).as_secs_f32();

            let seconds_since_start = (Instant::now() - start).as_secs();
            if seconds_since_start != total_seconds {
                println!(
                    "averaging {} fps",
                    1f32 / (total_time / total_frames as f32)
                );
                total_frames = 0;
                total_time = 0f32;
                total_seconds = seconds_since_start;
            }
        } else {
            break;
        }
    }

    capture.stop().expect("failed to stop capture");
}
