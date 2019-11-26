use crate::prelude::*;
use nu_protocol::{ShellError, Value};

use crate::parser::registry::Signature;

pub struct Echo;

impl PerItemCommand for Echo {
    fn name(&self) -> &str {
        "echo"
    }

    fn signature(&self) -> Signature {
        Signature::build("echo").rest(SyntaxShape::Any, "the values to echo")
    }

    fn usage(&self) -> &str {
        "Echo the arguments back to the user."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Value,
    ) -> Result<OutputStream, ShellError> {
        run(call_info, registry, raw_args)
    }
}

fn run(
    call_info: &CallInfo,
    _registry: &CommandRegistry,
    _raw_args: &RawCommandArgs,
) -> Result<OutputStream, ShellError> {
    let mut output = vec![];

    if let Some(ref positional) = call_info.args.positional {
        for i in positional {
            match i.as_string() {
                Ok(s) => {
                    output.push(Ok(ReturnSuccess::Value(
                        UntaggedValue::string(s).into_value(i.tag.clone()),
                    )));
                }
                _ => match i {
                    Value {
                        value: UntaggedValue::Table(table),
                        ..
                    } => {
                        for value in table {
                            output.push(Ok(ReturnSuccess::Value(value.clone())));
                        }
                    }
                    _ => {
                        output.push(Ok(ReturnSuccess::Value(i.clone())));
                    }
                },
            }
        }
    }

    let stream = VecDeque::from(output);

    Ok(stream.to_output_stream())
}
