#[cfg(feature = "clipboard")]
pub mod clipboard {
    use crate::commands::WholeStreamCommand;
    use crate::context::CommandRegistry;
    use crate::prelude::*;
    use futures::stream::StreamExt;
    use nu_protocol::{ReturnValue, ShellError, Signature, Value};

    use clipboard::{ClipboardContext, ClipboardProvider};

    pub struct Clip;

    #[derive(Deserialize)]
    pub struct ClipArgs {}

    impl WholeStreamCommand for Clip {
        fn name(&self) -> &str {
            "clip"
        }

        fn signature(&self) -> Signature {
            Signature::build("clip")
        }

        fn usage(&self) -> &str {
            "Copy the contents of the pipeline to the copy/paste buffer"
        }

        fn run(
            &self,
            args: CommandArgs,
            registry: &CommandRegistry,
        ) -> Result<OutputStream, ShellError> {
            args.process(registry, clip)?.run()
        }
    }

    pub fn clip(
        ClipArgs {}: ClipArgs,
        RunnableContext { input, name, .. }: RunnableContext,
    ) -> Result<OutputStream, ShellError> {
        let stream = async_stream! {
            let values: Vec<Value> = input.values.collect().await;

            let mut clip_stream = inner_clip(values, name).await;
            while let Some(value) = clip_stream.next().await {
                yield value;
            }
        };

        let stream: BoxStream<'static, ReturnValue> = stream.boxed();

        Ok(OutputStream::from(stream))
    }

    async fn inner_clip(input: Vec<Value>, name: Tag) -> OutputStream {
        let mut clip_context: ClipboardContext = ClipboardProvider::new().unwrap();
        let mut new_copy_data = String::new();

        if input.len() > 0 {
            let mut first = true;
            for i in input.iter() {
                if !first {
                    new_copy_data.push_str("\n");
                } else {
                    first = false;
                }

                let string: String = match i.as_string() {
                    Ok(string) => string.to_string(),
                    Err(_) => {
                        return OutputStream::one(Err(ShellError::labeled_error(
                            "Given non-string data",
                            "expected strings from pipeline",
                            name,
                        )))
                    }
                };

                new_copy_data.push_str(&string);
            }
        }

        clip_context.set_contents(new_copy_data).unwrap();

        OutputStream::empty()
    }
}
