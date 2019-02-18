Aim of this library is to provide convenient way of adding statically typed context information to errors in Rust.

This crate provides two ways of adding context:
* to new error types by means of `WithContext` trait,
* to existing errors by wrapping in `ErrorContext` type and converting to your type using `From` trait.

It provides extension methods for `Result` type as well as some free functions to help with adding context.

For examples and usage see crate documentation at [docs.rs](https://docs.rs/error-context).

If you are looking for more dynamic way of adding context to error messages see [problem crate](https://github.com/jpastuszek/problem).