use colorsys::ParseError;
use config::ConfigError;
use govee_rs;

pub type Result<T> = std::result::Result<T, SpiritError>;

/// SpiritError enumerates all possible errors returned by this library
#[derive(Debug)]
pub enum SpiritError {
    Error(String),

    /// Represents all other cases of GoveeError
    ConfigError(ConfigError),

    /// Represents env VarErrors
    EnvError(std::env::VarError),

    /// Represents all other cases of GoveeError
    GoveeError(govee_rs::error::GoveeError),

    /// Represents all other cases of IO error
    IOError(std::io::Error),

    /// Represents colorsys parse errors
    ParseError(ParseError),
}

impl std::error::Error for SpiritError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            SpiritError::ConfigError(ref err) => Some(err),
            SpiritError::Error(_) => None,
            SpiritError::EnvError(ref err) => Some(err),
            SpiritError::GoveeError(ref err) => Some(err),
            SpiritError::IOError(ref err) => Some(err),
            SpiritError::ParseError(ref err) => Some(err),
        }
    }
}

impl std::fmt::Display for SpiritError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            SpiritError::Error(ref msg) => write!(f, "{}", msg),
            SpiritError::ConfigError(ref err) => err.fmt(f),
            SpiritError::EnvError(ref err) => err.fmt(f),
            SpiritError::GoveeError(ref err) => err.fmt(f),
            SpiritError::IOError(ref err) => err.fmt(f),
            SpiritError::ParseError(ref err) => err.fmt(f),
        }
    }
}

impl From<ConfigError> for SpiritError {
    fn from(err: ConfigError) -> SpiritError {
        SpiritError::ConfigError(err)
    }
}

impl From<std::env::VarError> for SpiritError {
    fn from(err: std::env::VarError) -> SpiritError {
        SpiritError::EnvError(err)
    }
}

impl From<govee_rs::error::GoveeError> for SpiritError {
    fn from(err: govee_rs::error::GoveeError) -> SpiritError {
        SpiritError::GoveeError(err)
    }
}

impl From<std::io::Error> for SpiritError {
    fn from(err: std::io::Error) -> SpiritError {
        SpiritError::IOError(err)
    }
}

impl From<ParseError> for SpiritError {
    fn from(err: ParseError) -> SpiritError {
        SpiritError::ParseError(err)
    }
}

pub trait UnwrapOrExit<T>
where
    Self: Sized,
{
    fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce() -> T;

    fn unwrap_or_exit(self, message: &str) -> T {
        let err = clap::Error::with_description(message, clap::ErrorKind::InvalidValue);
        self.unwrap_or_else(|| err.exit())
    }
}

impl<T> UnwrapOrExit<T> for Option<T> {
    fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        self.unwrap_or_else(f)
    }
}

impl<T> UnwrapOrExit<T> for Result<T> {
    fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        self.unwrap_or_else(|_| f())
    }

    fn unwrap_or_exit(self, message: &str) -> T {
        self.unwrap_or_else(|e| {
            let err = clap::Error::with_description(
                &format!("{}: {}", message, e),
                clap::ErrorKind::InvalidValue,
            );
            err.exit()
        })
    }
}
