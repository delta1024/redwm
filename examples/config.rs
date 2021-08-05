use redwm::run;
use x11rb::connect;
use std::process::{exit, Command};

fn main() {

    {
	Command::new("/usr/bin/alacritty")
	    .spawn()
	    .expect("Failed to execute process");
    }

    match run(connect(None).expect("Failed to connect to server")) {
	Ok(()) => exit(0),
	Err(err) => {
	    eprintln!("{}", err);

	    exit(2);
	}
    }
}

