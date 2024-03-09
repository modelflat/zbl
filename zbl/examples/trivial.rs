use std::time::Instant;

use clap::Parser;
use opencv::{highgui, prelude::*};
use zbl::{Capturable, Capture, Display, Frame, Window};

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

    let mut capture = Capture::new(target, true).expect("failed to initialize capture");

    capture.start().expect("failed to start capture");

    highgui::named_window("Test", highgui::WINDOW_NORMAL | highgui::WINDOW_KEEPRATIO)
        .expect("failed to setup opencv window");

    let start = Instant::now();
    let mut prev = 0;
    let mut cnt = 0;
    let mut tt = 0f32;
    loop {
        let t = Instant::now();
        if let Some(Frame { texture, ptr }) = capture.grab().expect("failed to get frame") {
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

    capture.stop().expect("failed to stop capture");
}
