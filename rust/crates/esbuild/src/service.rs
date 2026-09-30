//! The esbuild service: a child process running `esbuild --service`, driven over its stdio
//! protocol. Everything under `service` uses only `std`.

mod client;
pub mod protocol;

pub use client::{
    BuildRequest, BuildResult, CallbackError, Hook, LoadArgs, LoadFn, Loaded, Location, Message,
    Note, OutputFile, Plugin, ResolveArgs, ResolveFn, Resolved, Service, ServiceError, Stdin,
    binary_version,
};
