use std::time::Instant;

use clap::Parser;
use opencv::{highgui, prelude::*};
use zbl::{ro_initialize_once, Capture, Window};

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(long)]
    window: String,
}

fn main() {
    ro_initialize_once();

    let args = Args::parse();
    let window = Window::find_first(&args.window).expect("failed to find window");
    let mut capturer = Capture::new(window).expect("failed to initialize capture");

    capturer.start().expect("failed to start capture");

    highgui::named_window("Test", highgui::WINDOW_AUTOSIZE).expect("failed to setup opencv window");

    let start = Instant::now();
    let mut prev = 0;
    let mut cnt = 0;
    let mut tt = 0f32;
    loop {
        let t = Instant::now();
        if let Some((texture, ptr)) = capturer.grab().expect("failed to get frame") {
            let mat = unsafe {
                Mat::new_size_with_data(
                    opencv::core::Size::new(texture.desc.Width as i32, texture.desc.Height as i32),
                    opencv::core::CV_8UC4,
                    ptr.pData,
                    ptr.RowPitch as usize,
                )
            }
            .expect("failed to convert to opencv frame");
            let t = Instant::now() - t;
            cnt += 1;
            tt += t.as_secs_f32();
            if (Instant::now() - start).as_secs() != prev {
                println!("averaging {} fps", 1f32 / (tt / cnt as f32));
                cnt = 0;
                tt = 0f32;
                prev = (Instant::now() - start).as_secs();
            }
            highgui::imshow("Test", &mat).expect("failed to show frame");
            if highgui::wait_key(8).expect("failed to wait user input") != -1 {
                break;
            }
        } else {
            break;
        }
    }

    capturer.stop().expect("failed to stop capture");
}
