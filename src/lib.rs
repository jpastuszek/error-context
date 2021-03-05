/*!
This crate provides methods and types that help with adding additional context information to error types.

# Usage
There are two ways to add context information to your error types:
1. by extending your error type with a field that will store the context information and then adding context to the error value,
2. wrapping any error type together with the context information and then converting this boundle to type that can store the error and context.

This crate provides types, traits and extension methods designed to help with the above tasks.
It is recommended to import all the types and traits via perlude module: `use error_type::prelude::*`.

## Adding context to types that can collect context
If your type can collect context information you can implement `WithContext` trait for it. By doing so you enable some of the provided extension methods to work with your type.

### Directly to value
You can add context to value of your error with `.with_context(context)`.

### To error wrapped in `Result`
Use `.error_while(context)` method on `Result` value to add context to error value of type that implements `WithContext`.

You can also use `in_context_of(context, closure)` function to add context to result of provided closure. You can use `?` within the closure to control the flow.

There is also `.error_while_with(context_function)` and `in_context_of_with(context_function, closure)` variants that can be used to defer construction of context to error path.

## Adding context to other types
External error types may not support adding context.
The `ErrorContext` type can be used to wrap error value and context information together. 
This type implements `WithContext` and adding further context information will result in wrapping with another layer of `ErrorContext` type.

The main use case for this method is to wrap error in one or more layers of context and then convert them to your own error type consuming 
the error and the context information using `From` trait.
This enables use of `?` to convert external error types with added context to your error type.

### Directly to value
You can wrap any type in `ErrorContext` type using `.wrap_context(context)` method.

### To error wrapped in `Result`
When working with `Result` value you can wrap error value in `ErrorContext` using `.wrap_error_while(context)`.

There is also `.wrap_error_while_with(context_function)` and `wrap_in_context_of_with(context_function, closure)` variants that can be used to defer construction of context to error path.

### Using `ErrorNoContext`
You can also use `.to_root_cause()` directly on error value or `.map_error_context()` on `Result` to wrap error type in `ErrorNoContext`.

Adding context information to `ErrorNoContext` converts it into `ErrorContext`. 
`ErrorNoContext` is intended to be used within function scope to enable functions and methods that work with `WithContext` to add 
context information bafore error is returned.

## Usage example
In this example we will create our own error type called `MyError`.
We will wrap extra context information to `std::io::Error` value using `.wrap_error_while(context)` and as another example using `.wrap_in_context_of(context, closure)`.
Finally by implementing `From<ErrorContext<io::Error, &'static str>>` for `MyError` we can use `?` operator to convert this error to `MyError` 
persisting the context information added.

```rust
use error_context::prelude::*;
use std::io;

enum MyError {
    IoError(io::Error, &'static str),
}

impl From<ErrorContext<io::Error, &'static str>> for MyError {
    fn from(error: ErrorContext<io::Error, &'static str>) -> MyError {
        MyError::IoError(error.error, error.context)
    }
}

fn work_with_file() -> Result<(), MyError> {
    Err(io::Error::new(io::ErrorKind::InvalidInput, "boom!"))
        .wrap_error_while("working with file")?;
    Ok(())
}

match work_with_file().unwrap_err() {
    MyError::IoError(_, "working with file") => (),
    _ => panic!("wrong context"),
}

fn do_stuff() -> Result<(), MyError> {
    wrap_in_context_of("doing stuff", || {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "boom!"))?;
        Ok(())
    })?;
    Ok(())
}

match do_stuff().unwrap_err() {
    MyError::IoError(_, "doing stuff") => (),
    _ => panic!("wrong context"),
}
```

# Usage guidelines
* Use error context to provide information about which good program path was taken that lead to an error, e.g: "while parsing filed x of message type y".
* Error context should provide detail for the end user who sees the error message and not be used to distinguish between two different errors in code - use `Display` types like `&'static str` as context type.
* Don't add errors or error path information to context - this should be part of the error type, in particular its `Display` and `Error::source` implementation.
* Don't add arguments of function call you are rising error from to the context - this should be responsibility of the caller - otherwise it would be difficult to
avoid non-`'static` references or allocations on error path and avoid showing sensitive data to end user, e.g. SQL query text or passwords.
* Don't put non-`'static` references to context or the error value cannot be bubbled up easily or returned as `Error::source`.
*/

use std::error::Error;
use std::fmt::Debug;
use std::fmt::{self, Display};

/// Includes `WithContext` trait, `ErrorContext`, `ErrorNoContext` types and related conversion traits and `*in_context_of*` functions
pub mod prelude {
    pub use crate::{
        in_context_of, in_context_of_with, wrap_in_context_of, wrap_in_context_of_with,
        ErrorContext, ErrorNoContext, MapErrorNoContext, ResultErrorWhile, ResultErrorWhileWrap,
        ToErrorNoContext, WithContext, WrapContext,
    };
}

/// Add context to object
pub trait WithContext<C> {
    type ContextError;
    fn with_context(self, context: C) -> Self::ContextError;
}

/// Add context to error carried by another type like `Result`
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

/// Wrap value in `ErrorNoContext` to add more context using `WithContext` trait that will convert it to `ErrorContext`
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

/// Wrap value with `ErrorNoContext`
pub trait ToErrorNoContext<T> {
    fn to_root_cause(self) -> ErrorNoContext<T>;
}

impl<T> ToErrorNoContext<T> for T {
    fn to_root_cause(self) -> ErrorNoContext<Self> {
        ErrorNoContext(self)
    }
}

/// Map error caring type by wrapping it's error value in `ErrorNoContext`
pub trait MapErrorNoContext<O, E> {
    fn map_error_context(self) -> Result<O, ErrorNoContext<E>>;
}

impl<O, E> MapErrorNoContext<O, E> for Result<O, E> {
    fn map_error_context(self) -> Result<O, ErrorNoContext<E>> {
        self.map_err(ToErrorNoContext::to_root_cause)
    }
}

/// Wrap error value together with context information
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

/// Wrap value in type with context information
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

/// `Result` extension trait to wrap error value in `ErrorContext` with given context information
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

/// Executes closure adding context to returned error value with `.with_context(context)`
pub fn in_context_of<O, E, C, CE, B>(context: C, body: B) -> Result<O, CE>
where
    E: WithContext<C, ContextError = CE>,
    B: FnOnce() -> Result<O, E>,
{
    body().map_err(|e| e.with_context(context))
}

/// Executes closure adding context to returned error value with `.with_context(context)` obtaining context by calling given function on error path
pub fn in_context_of_with<O, E, C, CE, F, M, B>(context: F, body: B) -> Result<O, CE>
where
    F: FnOnce() -> C,
    E: WithContext<C, ContextError = CE>,
    B: FnOnce() -> Result<O, E>,
{
    body().map_err(|e| e.with_context(context()))
}

/// Executes closure adding context to returned error value by wrapping it in `ErrorContext` with `.wrap_context(context)`
pub fn wrap_in_context_of<O, E, C, B>(context: C, body: B) -> Result<O, ErrorContext<E, C>>
where
    E: WrapContext<C, ContextError = ErrorContext<E, C>>,
    B: FnOnce() -> Result<O, E>,
{
    body().map_err(|e| e.wrap_context(context))
}

/// Executes closure adding context to returned error value by wrapping it in `ErrorContext` with `.wrap_context(context)` obtaining context by calling given function on error path
pub fn wrap_in_context_of_with<O, E, C, F, B>(
    context: F,
    body: B,
) -> Result<O, ErrorContext<E, C>>
where
    F: FnOnce() -> C,
    E: WrapContext<C, ContextError = ErrorContext<E, C>>,
    B: FnOnce() -> Result<O, E>,
{
    body().map_err(|e| e.wrap_context(context()))
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use assert_matches::*;
    use std::io;

    #[derive(Debug)]
    enum FooError {
        Foo {
            context: Vec<String>,
        },
        Bar {
            num: i32,
            context: Vec<String>,
        },
        IoError {
            error: io::Error,
            context: Vec<String>,
        },
    }

    impl WithContext<String> for FooError {
        type ContextError = Self;
        fn with_context(mut self, message: String) -> Self {
            match self {
                FooError::Foo {
                    ref mut context, ..
                } => context.push(message),
                FooError::Bar {
                    ref mut context, ..
                } => context.push(message),
                FooError::IoError {
                    ref mut context, ..
                } => context.push(message),
            }
            self
        }
    }

    impl From<ErrorContext<io::Error, String>> for FooError {
        fn from(error_context: ErrorContext<io::Error, String>) -> FooError {
            FooError::IoError {
                error: error_context.error,
                context: vec![error_context.context],
            }
        }
    }

    #[test]
    fn test_in_type_context() {
        let err: Result<(), FooError> = Err(FooError::Foo {
            context: Vec::new(),
        });
        assert_matches!(err.error_while("doing stuff".to_string()), Err(FooError::Foo { context }) => assert_eq!(context, vec!["doing stuff".to_string()]));

        let err: Result<(), FooError> = Err(FooError::Bar {
            num: 1,
            context: Vec::new(),
        });
        assert_matches!(err.error_while("doing stuff".to_string()), Err(FooError::Bar { num: 1, context }) => assert_eq!(context, vec!["doing stuff".to_string()]));
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
            let err: Result<(), FooError> = Err(FooError::Foo {
                context: Vec::new(),
            });
            err
        });

        assert_matches!(err.error_while("doing other stuff".to_string()), Err(FooError::Foo { context: c }) => assert_eq!(c, vec!["doing stuff".to_string(), "doing other stuff".to_string()]));
    }

    #[test]
    fn test_wrap_in_context_of_type_context() {
        fn foo() -> Result<(), FooError> {
            wrap_in_context_of("doing stuff".to_string(), || {
                Err(io::Error::new(io::ErrorKind::InvalidInput, "boom!"))?;
                Ok(())
            })?;
            Ok(())
        }

        assert_matches!(foo().error_while("doing other stuff".to_string()), Err(FooError::IoError { context, .. }) => assert_eq!(context, vec!["doing stuff".to_string(), "doing other stuff".to_string()]));
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
