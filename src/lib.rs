use std::error::Error;
use std::fmt::{self, Display};
use std::fmt::Debug;

pub trait WithContext<C> {
    type ContextError;
    fn with_context(self, context: C) -> Self::ContextError;
}

pub trait ResultErrorWhile<C> {
    type ContextError;
    fn error_while(self, context: C) -> Self::ContextError;
} 

impl<O, E, C> ResultErrorWhile<C> for Result<O, E> where E: WithContext<C, ContextError = E> {
    type ContextError = Self;
    fn error_while(self, context: C) -> Self {
        self.map_err(|e| e.with_context(context))
    }
}

#[derive(Debug)]
pub struct ErrorContext<E, C>(E, Option<C>);

impl<E, C> ErrorContext<E, C> {
    pub fn unwrap(self) -> E {
        self.0
    }
}

impl<E, C> Display for ErrorContext<E, C> where E: Display, C: Display {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(context) = &self.1 {
            write!(f, "while {} got error: {}", context, self.0)
        } else {
            self.0.fmt(f)
        }
    }
}

impl<E, C> Error for ErrorContext<E, C> where E: Error, C: Display + Debug {
    fn description(&self) -> &str {
        self.0.description()
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

impl<E, C> WithContext<C> for ErrorContext<E, C> {
    type ContextError = ErrorContext<E, C>;
    fn with_context(self, context: C) -> ErrorContext<E, C> {
        ErrorContext(self.0, Some(context))
    }
}

pub trait WrapContext<C> {
    type ContextError;
    fn wrap_context(self) -> Self::ContextError;
}

impl<E, C> WrapContext<C> for E where E: Error {
    type ContextError = ErrorContext<E, C>;
    fn wrap_context(self) -> ErrorContext<E, C> {
        ErrorContext(self, None)
    }
}

pub trait MapErrorContext<O, E, C> {
    fn map_error_context(self) -> Result<O, ErrorContext<E, C>>;
}

impl<O, E, C> MapErrorContext<O, E, C> for Result<O, E> where E: WrapContext<C> {
    fn map_error_context(self) -> Result<O, ErrorContext<E, C>> {
        // TODO: chain
        self.map_err(|e| ErrorContext(e, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::*;

    #[derive(Debug)]
    enum FooError {
        Foo { context: Option<String> },
        Bar { num: i32, ctx: Option<String> },
    }

    impl WithContext<String> for FooError  {
        type ContextError = Self;
        fn with_context(self, context: String) -> Self {
            match self {
                FooError::Foo { .. } => FooError::Foo { context: Some(context) },
                FooError::Bar { num, .. } => FooError::Bar { num, ctx: Some(context) },
            }
        }
    }

    #[test]
    fn test_in_type_context() {
        let err: Result<(), FooError> = Err(FooError::Foo { context: None });
        assert_matches!(err.error_while("doing stuff".to_string()), Err(FooError::Foo { context: Some(c) }) => assert_eq!(c, "doing stuff".to_string()));

        let err: Result<(), FooError> = Err(FooError::Bar { num: 1, ctx: None });
        assert_matches!(err.error_while("doing stuff".to_string()), Err(FooError::Bar { num: 1, ctx: Some(c) }) => assert_eq!(c, "doing stuff".to_string()));
    }

    #[test]
    fn test_wrapped_context() {
        use std::io::{Error, ErrorKind};
        let err: Result<(), Error> = Err(Error::new(ErrorKind::Other, "oh no!"));

        assert_eq!(err.map_error_context().error_while("doing stuff".to_string()).unwrap_err().to_string(), "while doing stuff got error: oh no!");
    }
}
