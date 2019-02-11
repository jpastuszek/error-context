/*!
    * Use error context to provide information about wich good program path was taken that in the end lead to error,
    * Don't add errors or error path information to context - this should be part of the error type, in particular its `Display` and `Error::source` implementation,
    * Don't add arguments of function call you are rising error from to the context - this should be responsibility of the caller,
 */

use std::error::Error;
use std::fmt::Debug;
use std::fmt::{self, Display};

pub mod prelude {
    pub use crate::{
        in_context_of, in_context_of_with, ErrorContext, ErrorNoContext, MapErrorNoContext,
        ResultErrorWhile, ResultErrorWhileWrap, ToErrorNoContext, WithContext, WrapContext,
    };
}

pub trait WithContext<C> {
    type ContextError;
    fn with_context(self, context: C) -> Self::ContextError;
}

pub trait ResultErrorWhile<C> {
    type ContextError;
    fn error_while(self, context: C) -> Self::ContextError;
    fn error_while_with<F>(self, context: F) -> Self::ContextError
    where
        F: FnOnce() -> C;
}

impl<O, E, C> ResultErrorWhile<C> for Result<O, E>
where
    E: WithContext<C, ContextError = E>,
{
    type ContextError = Self;
    fn error_while(self, context: C) -> Self {
        self.map_err(|e| e.with_context(context))
    }

    fn error_while_with<F>(self, context: F) -> Self::ContextError
    where
        F: FnOnce() -> C,
    {
        self.map_err(|e| e.with_context(context()))
    }
}

#[derive(Debug)]
pub struct ErrorNoContext<E>(pub E);

impl<E> Display for ErrorNoContext<E>
where
    E: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<E> Error for ErrorNoContext<E>
where
    E: Error,
{
    fn description(&self) -> &str {
        self.0.description()
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

impl<E, C> WithContext<C> for ErrorNoContext<E> {
    type ContextError = ErrorContext<E, C>;
    fn with_context(self, context: C) -> ErrorContext<E, C> {
        ErrorContext {
            error: self.0,
            context,
        }
    }
}

pub trait ToErrorNoContext<T> {
    fn to_root_cause(self) -> ErrorNoContext<T>;
}

impl<T> ToErrorNoContext<T> for T {
    fn to_root_cause(self) -> ErrorNoContext<Self> {
        ErrorNoContext(self)
    }
}

pub trait MapErrorNoContext<O, E> {
    fn map_error_context(self) -> Result<O, ErrorNoContext<E>>;
}

impl<O, E> MapErrorNoContext<O, E> for Result<O, E> {
    fn map_error_context(self) -> Result<O, ErrorNoContext<E>> {
        self.map_err(ToErrorNoContext::to_root_cause)
    }
}

#[derive(Debug)]
pub struct ErrorContext<E, C> {
    pub error: E,
    pub context: C,
}

impl<E, C> Display for ErrorContext<E, C>
where
    E: Display,
    C: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "while {} got error: {}", self.context, self.error)
    }
}

impl<E, C> Error for ErrorContext<E, C>
where
    E: Error,
    C: Display + Debug,
{
    fn description(&self) -> &str {
        self.error.description()
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error.source()
    }
}

impl<E, C, C2> WithContext<C2> for ErrorContext<E, C> {
    type ContextError = ErrorContext<ErrorContext<E, C>, C2>;
    fn with_context(self, context: C2) -> ErrorContext<ErrorContext<E, C>, C2> {
        ErrorContext {
            error: self,
            context,
        }
    }
}

pub trait WrapContext<C> {
    type ContextError;
    fn wrap_context(self, context: C) -> Self::ContextError;
}

impl<E, C> WrapContext<C> for E {
    type ContextError = ErrorContext<E, C>;
    fn wrap_context(self, context: C) -> ErrorContext<E, C> {
        ErrorContext {
            error: self,
            context,
        }
    }
}

pub trait ResultErrorWhileWrap<O, E, C> {
    fn wrap_error_while(self, context: C) -> Result<O, ErrorContext<E, C>>;
    fn wrap_error_while_with<F>(self, context: F) -> Result<O, ErrorContext<E, C>>
    where
        F: FnOnce() -> C;
}

impl<O, E, C> ResultErrorWhileWrap<O, E, C> for Result<O, E>
where
    E: WrapContext<C, ContextError = ErrorContext<E, C>>,
{
    fn wrap_error_while(self, context: C) -> Result<O, ErrorContext<E, C>> {
        self.map_err(|e| e.wrap_context(context))
    }

    fn wrap_error_while_with<F>(self, context: F) -> Result<O, ErrorContext<E, C>>
    where
        F: FnOnce() -> C,
    {
        self.map_err(|e| e.wrap_context(context()))
    }
}

/// Executes closure with with_context context
pub fn in_context_of<O, E, C, CE, B>(context: C, body: B) -> Result<O, CE>
where
    E: WithContext<C, ContextError = CE>,
    B: FnOnce() -> Result<O, E>,
{
    body().map_err(|e| e.with_context(context))
}

/// Executes closure with with_context context function called on Err wariant
pub fn in_context_of_with<O, E, C, CE, F, M, B>(context: F, body: B) -> Result<O, CE>
where
    F: FnOnce() -> C,
    E: WithContext<C, ContextError = CE>,
    B: FnOnce() -> Result<O, E>,
{
    body().map_err(|e| e.with_context(context()))
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

    impl WithContext<String> for FooError {
        type ContextError = Self;
        fn with_context(self, context: String) -> Self {
            match self {
                FooError::Foo { .. } => FooError::Foo {
                    context: Some(context),
                },
                FooError::Bar { num, .. } => FooError::Bar {
                    num,
                    ctx: Some(context),
                },
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

        assert_eq!(
            err.wrap_error_while("doing stuff".to_string())
                .unwrap_err()
                .to_string(),
            "while doing stuff got error: oh no!"
        );
    }

    #[test]
    fn test_wrapped_context_nested() {
        use std::io::{Error, ErrorKind};
        let err: Result<(), Error> = Err(Error::new(ErrorKind::Other, "file is no good"));

        assert_eq!(
            err.wrap_error_while("opening file".to_string())
                .wrap_error_while("processing fish sticks".to_string())
                .unwrap_err()
                .to_string(),
            "while processing fish sticks got error: while opening file got error: file is no good"
        );
    }

    #[test]
    fn test_in_context_of_type_context() {
        let err = in_context_of("doing stuff".to_string(), || {
            let err: Result<(), FooError> = Err(FooError::Foo { context: None });
            err
        });

        assert_matches!(err.error_while("doing stuff".to_string()), Err(FooError::Foo { context: Some(c) }) => assert_eq!(c, "doing stuff".to_string()));
    }

    #[test]
    fn test_in_context_of_wrapped_context() {
        use std::io::{Error, ErrorKind};

        let err = in_context_of("opening file".to_string(), || {
            let err: Result<(), Error> = Err(Error::new(ErrorKind::Other, "file is no good"));
            err.map_error_context()
        });

        assert_eq!(
            err.wrap_error_while("processing fish sticks".to_string())
                .unwrap_err()
                .to_string(),
            "while processing fish sticks got error: while opening file got error: file is no good"
        );
    }
}
