use crate::maybe_owned::MaybeOwned;
use crate::value::primitive::Primitive;
use crate::value::{UntaggedValue, Value};
use derive_new::new;
use getset::Getters;
use indexmap::IndexMap;
use nu_source::{b, DebugDocBuilder, PrettyDebug, Spanned, Tag};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, Clone, Getters, new)]
pub struct Dictionary {
    #[get = "pub"]
    pub entries: IndexMap<String, Value>,
}

#[derive(Debug, new)]
struct DebugEntry<'a> {
    key: &'a str,
    value: &'a Value,
}

impl<'a> PrettyDebug for DebugEntry<'a> {
    fn pretty(&self) -> DebugDocBuilder {
        (b::key(self.key.to_string()) + b::equals() + self.value.pretty().as_value()).group()
    }
}

impl PrettyDebug for Dictionary {
    fn pretty(&self) -> DebugDocBuilder {
        b::delimit(
            "(",
            b::intersperse(
                self.entries()
                    .iter()
                    .map(|(key, value)| DebugEntry::new(key, value)),
                b::space(),
            ),
            ")",
        )
    }
}

impl From<IndexMap<String, Value>> for Dictionary {
    fn from(input: IndexMap<String, Value>) -> Dictionary {
        let mut out = IndexMap::default();

        for (key, value) in input {
            out.insert(key, value);
        }

        Dictionary::new(out)
    }
}

impl Dictionary {
    pub fn get_data(&self, desc: &String) -> MaybeOwned<'_, Value> {
        match self.entries.get(desc) {
            Some(v) => MaybeOwned::Borrowed(v),
            None => MaybeOwned::Owned(
                UntaggedValue::Primitive(Primitive::Nothing).into_untagged_value(),
            ),
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    pub(crate) fn get_data_by_key(&self, name: Spanned<&str>) -> Option<Value> {
        let result = self
            .entries
            .iter()
            .find(|(desc_name, _)| *desc_name == name.item)?
            .1;

        Some(
            result
                .value
                .clone()
                .into_value(Tag::new(result.tag.anchor(), name.span)),
        )
    }

    pub(crate) fn get_mut_data_by_key(&mut self, name: &str) -> Option<&mut Value> {
        match self
            .entries
            .iter_mut()
            .find(|(desc_name, _)| *desc_name == name)
        {
            Some((_, v)) => Some(v),
            None => None,
        }
    }

    pub(crate) fn insert_data_at_key(&mut self, name: &str, value: Value) {
        self.entries.insert(name.to_string(), value);
    }
}
