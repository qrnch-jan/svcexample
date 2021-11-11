use std::fmt;
use std::io;

use qargparser as ap;

#[derive(Debug)]
pub enum Error {
  ArgParser(String),
  EventLog(String),
  Figment(String),
  IO(String),
  Registry(String),
  WindowsService(String)
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match &*self {
      Error::ArgParser(s) => {
        write!(f, "ArgParser error; {}", s)
      }
      Error::EventLog(s) => {
        write!(f, "EventLog error; {}", s)
      }
      Error::Figment(s) => {
        write!(f, "figment error; {}", s)
      }
      Error::IO(s) => write!(f, "I/O error; {}", s),
      Error::Registry(s) => write!(f, "registry; {}", s),
      Error::WindowsService(s) => write!(f, "windows-service error; {}", s)
    }
  }
}

impl<T> From<ap::ErrKind<T>> for Error {
  fn from(err: ap::ErrKind<T>) -> Self {
    Error::ArgParser(err.to_string())
  }
}

impl From<eventlog::Error> for Error {
  fn from(err: eventlog::Error) -> Self {
    Error::EventLog(err.to_string())
  }
}

impl From<figment::Error> for Error {
  fn from(err: figment::Error) -> Self {
    Error::Figment(err.to_string())
  }
}

impl From<io::Error> for Error {
  fn from(err: io::Error) -> Self {
    Error::IO(err.to_string())
  }
}

impl From<registry::key::Error> for Error {
  fn from(err: registry::key::Error) -> Self {
    Error::Registry(err.to_string())
  }
}

impl From<windows_service::Error> for Error {
  fn from(err: windows_service::Error) -> Self {
    Error::WindowsService(err.to_string())
  }
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
