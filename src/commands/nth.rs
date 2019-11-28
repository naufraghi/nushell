use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_protocol::{Signature, SyntaxShape};
use nu_errors::ShellError;
use nu_source::Tagged;

#[derive(Deserialize)]
struct NthArgs {
    amount: Tagged<i64>,
}

pub struct Nth;

impl WholeStreamCommand for Nth {
    fn name(&self) -> &str {
        "nth"
    }

    fn signature(&self) -> Signature {
        Signature::build("nth").required(
            "row number",
            SyntaxShape::Any,
            "the number of the row to return",
        )
    }

    fn usage(&self) -> &str {
        "Return only the selected row"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, nth)?.run()
    }
}

fn nth(
    NthArgs { amount }: NthArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    Ok(OutputStream::from_input(
        input.values.skip(amount.item as u64).take(1),
    ))
}
