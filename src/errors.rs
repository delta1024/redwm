use std::fmt;
use std::error::Error;

#[derive(Debug, Clone)]
pub struct ConnectionError;

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "No X11 Connection")
    }
}

impl Error for ConnectionError { }

#[derive(Debug, Clone)]
pub struct FontGCError;

impl fmt::Display for FontGCError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
	write!(f, "No Font Graphical Context")
    }
}

impl Error for FontGCError { }

#[derive(Debug, Clone)]
pub struct ScreenError;


impl fmt::Display for ScreenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
	write!(f, "could not get screen information")
    }
}

impl Error for ScreenError { }

#[derive(Debug, Clone)]
pub struct ScanError;

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
	write!(f, "there was an error grabbing the connection while scannig for windows")
    }
} 

impl Error for ScanError { }
