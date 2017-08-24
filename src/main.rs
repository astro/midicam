extern crate opencv;
extern crate midir;

use std::thread;

use opencv::core as cv;
use opencv::sys::types as cvtypes;
use opencv::{imgproc, objdetect, highgui};

use midir::{MidiInput, MidiOutput, Ignore};
use midir::os::unix::{VirtualInput, VirtualOutput};

//static mut the_matrix : Option<cv::Mat> = None; // : opencv::core::Mat<opencv::core::Mat<Vec3b>;

struct Cursor {
	x : i32,
	y : i32
}

fn run_webcam(matrix : &cv::Mat, cursor : &Cursor) -> Result<(), String> {
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
			let dest = opencv::core::Size {width: 16, height: 16};
			try!(imgproc::resize(&image0, &mut matrix, dest, 0.0, 0.0, 0));

			unsafe {
				let row = matrix.ptr(y).unwrap();
				let mut pix = row.offset(3 * cursor.x as isize);
				
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

fn midi_worker(matrix : &cv::Mat, cursor : &Cursor) {
	let midi_out = MidiOutput::new("MidiCam").unwrap();
	let mut conn_out = (midi_out.create_virtual("midicam").map_err(|e| e.kind())).unwrap();
	
	// Load Piano
	conn_out.send(&[192, 0]);

	loop {
		//let note = *pix / 2;
		let mut row = matrix.ptr(cursor.y).unwrap();
		unsafe {
			let mut pix = row.offset(3 * cursor.x as isize);
			let note = *pix / 2;
			
			println!("Sending Midi note {}", note);
			
			conn_out.send(&[128, note, 0]);
			conn_out.send(&[144, note, 127]);
			
			cursor.x = (cursor.x + 1) % 16;
			if cursor.x == 0 {
				cursor.y = (cursor.y + 1) % 16;
			}
		}
	}
}

fn main() {
	let mut cursor = Cursor {x: 0, y: 0};

	let the_matrix = cv::mat();

	thread::spawn(move || {
		println!("I'm in a thread, doing stuff");
		midi_worker(&the_matrix, &cursor);
	});

    run_webcam(&the_matrix, &cursor).unwrap()
}
