#[macro_export]
macro_rules! return_err {
    ($expr:expr) => {
        match $expr {
            Err(_) => return,
            Ok(expr) => expr,
        };
    };
}

#[macro_export]
macro_rules! stream {
    ($($expr:expr),*) => {{
        let mut v = VecDeque::new();

        $(
            v.push_back($expr);
        )*

        v
    }}
}

#[macro_export]
macro_rules! trace_stream {
    (target: $target:tt, $desc:tt = $expr:expr) => {{
        if log::log_enabled!(target: $target, log::Level::Trace) {
            use futures::stream::StreamExt;

            let objects = $expr.values.inspect(move |o| {
                trace!(
                    target: $target,
                    "{} = {}",
                    $desc,
                    nu_source::PrettyDebug::plain_string(o, 70)
                );
            });

            $crate::stream::InputStream::from_stream(objects.boxed())
        } else {
            $expr
        }
    }};
}

#[macro_export]
macro_rules! trace_out_stream {
    (target: $target:tt, $desc:tt = $expr:expr) => {{
        if log::log_enabled!(target: $target, log::Level::Trace) {
            use futures::stream::StreamExt;

            let objects = $expr.values.inspect(move |o| {
                trace!(
                    target: $target,
                    "{} = {}",
                    $desc,
                    match o {
                        Err(err) => format!("{:?}", err),
                        Ok(value) => value.display(),
                    }
                );
            });

            $crate::stream::OutputStream::new(objects)
        } else {
            $expr
        }
    }};
}

#[macro_export]
macro_rules! dict {
    ($( $key:expr => $value:expr ),*) => {
        $crate::data::dict::TaggedDictBuilder::build(Tag::unknown(), |d| {
            $(
                d.insert_untagged($key, $value);
            )*
        })
    };

    ([tag] => $tag:expr, $( $key:expr => $value:expr ),*) => {
        $crate::data::dict::TaggedDictBuilder::build($tag, |d| {
            $(
                d.insert_untagged($key, $value);
            )*
        })
    }
}

pub(crate) use nu_protocol::{errln, outln};

pub(crate) use crate::cli::MaybeOwned;
pub(crate) use crate::commands::command::{
    CallInfo, CommandAction, CommandArgs, ReturnSuccess, ReturnValue, RunnableContext,
};
pub(crate) use crate::commands::PerItemCommand;
pub(crate) use crate::commands::RawCommandArgs;
pub(crate) use crate::context::CommandRegistry;
pub(crate) use crate::context::Context;
pub(crate) use crate::data::types::ExtractType;
pub(crate) use crate::env::host::handle_unexpected;
pub(crate) use crate::env::Host;
pub(crate) use crate::errors::{CoerceInto, ParseError, ShellError};
pub(crate) use crate::parser::hir::SyntaxShape;
pub(crate) use crate::parser::parse::parser::Number;
pub(crate) use crate::shell::filesystem_shell::FilesystemShell;
pub(crate) use crate::shell::help_shell::HelpShell;
pub(crate) use crate::shell::shell_manager::ShellManager;
pub(crate) use crate::shell::value_shell::ValueShell;
pub(crate) use crate::stream::{InputStream, OutputStream};
pub(crate) use crate::traits::{ShellTypeName, SpannedTypeName};
pub(crate) use async_stream::stream as async_stream;
pub(crate) use bigdecimal::BigDecimal;
pub(crate) use futures::stream::BoxStream;
pub(crate) use futures::{FutureExt, Stream, StreamExt};
pub(crate) use nu_source::{
    b, AnchorLocation, DebugDocBuilder, HasFallibleSpan, HasSpan, PrettyDebug,
    PrettyDebugWithSource, Span, SpannedItem, Tag, TaggedItem, Text,
};
pub(crate) use num_bigint::BigInt;
pub(crate) use num_traits::cast::{FromPrimitive, ToPrimitive};
pub(crate) use num_traits::identities::Zero;
pub(crate) use serde::Deserialize;
pub(crate) use std::collections::VecDeque;
pub(crate) use std::future::Future;
pub(crate) use std::sync::{Arc, Mutex};

pub(crate) use itertools::Itertools;

pub trait FromInputStream {
    fn from_input_stream(self) -> OutputStream;
}

impl<T> FromInputStream for T
where
    T: Stream<Item = Value> + Send + 'static,
{
    fn from_input_stream(self) -> OutputStream {
        OutputStream {
            values: self.map(ReturnSuccess::value).boxed(),
        }
    }
}

pub trait ToInputStream {
    fn to_input_stream(self) -> InputStream;
}

impl<T, U> ToInputStream for T
where
    T: Stream<Item = U> + Send + 'static,
    U: Into<Result<Value, ShellError>>,
{
    fn to_input_stream(self) -> InputStream {
        InputStream {
            values: self.map(|item| item.into().unwrap()).boxed(),
        }
    }
}

pub trait ToOutputStream {
    fn to_output_stream(self) -> OutputStream;
}

impl<T, U> ToOutputStream for T
where
    T: Stream<Item = U> + Send + 'static,
    U: Into<ReturnValue>,
{
    fn to_output_stream(self) -> OutputStream {
        OutputStream {
            values: self.map(|item| item.into()).boxed(),
        }
    }
}
