use redwm::run;
use std::process::{exit, Command};
use x11rb::connect;

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
