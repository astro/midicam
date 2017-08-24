extern crate opencv;
extern crate midir;

use std::thread;
use std::thread::sleep;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use opencv::core as cv;
use opencv::sys::types as cvtypes;
use opencv::{imgproc, objdetect, highgui};

use midir::{MidiInput, MidiOutput, Ignore};
use midir::os::unix::{VirtualInput, VirtualOutput};

struct Shared<T>(Arc<RwLock<T>>);

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(self.0.clone())
    }
}

unsafe impl<T> Send for Shared<T> {}

impl<T> Shared<T> {
    pub fn new (t: T) -> Self {
        Shared(Arc::new(RwLock::new(t)))
    }
}

//static mut the_matrix : Option<cv::Mat> = None; // : opencv::core::Mat<opencv::core::Mat<Vec3b>;

struct Cursor {
    x : i32,
    y : i32
}

const SIZE: i32 = 16;
const DELAY: u32 = 100;

fn run_webcam(shared_matrix : Shared<cv::Mat>, shared_cursor : Shared<Cursor>) -> Result<(), String> {
    let mut classifier =
	try!(objdetect::CascadeClassifier::for_file("haarcascade_frontalface_alt.xml"));

    let mut capture = try!(highgui::VideoCapture::for_device(-1));
    try!(highgui::namedWindow("midicam", 0));

    let mut x = 0;
    let mut y = 0;

    loop {
	if try!(capture.grab()) {
	    let mut image0 = cv::mat();
	    try!(capture.retrieve(&mut image0, 0));
	    //let mut image1 = cv::mat();
	    let dest = opencv::core::Size {width: SIZE, height: SIZE};
            let mut matrix = shared_matrix.0.write().unwrap();
	    try!(imgproc::resize(&image0, &mut matrix, dest, 0.0, 0.0, 0));

            let cursor = shared_cursor.0.read().unwrap();
	    unsafe {
		let row = matrix.ptr(y).unwrap();
		let mut pix = row.offset(3 * (cursor.x + cursor.y * SIZE) as isize);

		*pix = 0xff;
		*pix.offset(1) = 0xff;
		*pix.offset(2) = 0xff;
	    }

	    try!(highgui::imshow("midicam", &matrix));
	}

	if try!(highgui::waitKey(10)) == 27 { break }
    }
    try!(highgui::destroyWindow("midicam"));
    try!(capture.release());
    Ok(())
}

fn midi_worker(shared_matrix : Shared<cv::Mat>, shared_cursor : Shared<Cursor>) {
    let midi_out = MidiOutput::new("MidiCam").unwrap();
    let mut conn_out = (midi_out.create_virtual("midicam").map_err(|e| e.kind())).unwrap();

    // Load Piano
    conn_out.send(&[192, 0]);

    loop {
        let note = {
            let matrix = shared_matrix.0.read().unwrap();
            let cursor = shared_cursor.0.read().unwrap();
	    //let note = *pix / 2;
	    let mut row = matrix.ptr(cursor.y).unwrap();
	    unsafe {
		let mut pix = row.offset(3 * cursor.x as isize);
		*pix / 2
            }
        };

	println!("Sending Midi note {}", note);

	conn_out.send(&[128, note, 0]);
        sleep(Duration::new(0, DELAY * 1000000));
	conn_out.send(&[144, note, 127]);

        let mut cursor = shared_cursor.0.write().unwrap();
	cursor.x = (cursor.x + 1) % SIZE;
	if cursor.x == 0 {
	    cursor.y = (cursor.y + 1) % SIZE;
	}
    }
}

fn main() {
    let cursor = Shared::new(Cursor {x: 0, y: 0});
    let cursor_ = cursor.clone();
    let the_matrix = Shared::new(cv::Mat::for_rows_and_cols(SIZE, SIZE, cv::CV_8UC3).unwrap());
    let the_matrix_ = the_matrix.clone();
    thread::spawn(move || {
	println!("I'm in a thread, doing stuff");
	midi_worker(the_matrix_, cursor_);
    });

    run_webcam(the_matrix, cursor).unwrap()
}
