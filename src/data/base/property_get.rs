use crate::parser::hir::path::{PathMember, UnspannedPathMember};
use crate::prelude::*;
use crate::ColumnPath;
use crate::SpannedTypeName;
use nu_protocol::ExpectedRange;
use nu_source::{Spanned, SpannedItem, Tagged};

impl Value {
    pub(crate) fn get_data_by_member(&self, name: &PathMember) -> Result<Value, ShellError> {
        match &self.value {
            // If the value is a row, the member is a column name
            UntaggedValue::Row(o) => match &name.unspanned {
                // If the member is a string, get the data
                UnspannedPathMember::String(string) => o
                    .get_data_by_key(string[..].spanned(name.span))
                    .ok_or_else(|| {
                        ShellError::missing_property(
                            "row".spanned(self.tag.span),
                            string.spanned(name.span),
                        )
                    }),

                // If the member is a number, it's an error
                UnspannedPathMember::Int(_) => Err(ShellError::invalid_integer_index(
                    "row".spanned(self.tag.span),
                    name.span,
                )),
            },

            // If the value is a table
            UntaggedValue::Table(l) => {
                match &name.unspanned {
                    // If the member is a string, map over the member
                    UnspannedPathMember::String(string) => {
                        let mut out = vec![];

                        for item in l {
                            match item {
                                Value {
                                    value: UntaggedValue::Row(o),
                                    ..
                                } => match o.get_data_by_key(string[..].spanned(name.span)) {
                                    Some(v) => out.push(v),
                                    None => {}
                                },
                                _ => {}
                            }
                        }

                        if out.len() == 0 {
                            Err(ShellError::missing_property(
                                "table".spanned(self.tag.span),
                                string.spanned(name.span),
                            ))
                        } else {
                            Ok(UntaggedValue::Table(out)
                                .into_value(Tag::new(self.anchor(), name.span)))
                        }
                    }
                    UnspannedPathMember::Int(int) => {
                        let index = int.to_usize().ok_or_else(|| {
                            ShellError::range_error(
                                ExpectedRange::Usize,
                                &"massive integer".spanned(name.span),
                                "indexing",
                            )
                        })?;

                        match self.get_data_by_index(index.spanned(self.tag.span)) {
                            Some(v) => Ok(v.clone()),
                            None => Err(ShellError::range_error(
                                0..(l.len()),
                                &int.spanned(name.span),
                                "indexing",
                            )),
                        }
                    }
                }
            }
            other => Err(ShellError::type_error(
                "row or table",
                other.type_name().spanned(self.tag.span),
            )),
        }
    }

    pub fn get_data_by_column_path(
        &self,
        path: &ColumnPath,
        callback: Box<dyn FnOnce((&Value, &PathMember, ShellError)) -> ShellError>,
    ) -> Result<Value, ShellError> {
        let mut current = self.clone();

        for p in path.iter() {
            let value = current.get_data_by_member(p);

            match value {
                Ok(v) => current = v.clone(),
                Err(e) => return Err(callback((&current.clone(), &p.clone(), e))),
            }
        }

        Ok(current)
    }

    pub fn insert_data_at_path(&self, path: &str, new_value: Value) -> Option<Value> {
        let mut new_obj = self.clone();

        let split_path: Vec<_> = path.split(".").collect();

        if let UntaggedValue::Row(ref mut o) = new_obj.value {
            let mut current = o;

            if split_path.len() == 1 {
                // Special case for inserting at the top level
                current.entries.insert(
                    path.to_string(),
                    new_value.value.clone().into_value(&self.tag),
                );
                return Some(new_obj);
            }

            for idx in 0..split_path.len() {
                match current.entries.get_mut(split_path[idx]) {
                    Some(next) => {
                        if idx == (split_path.len() - 2) {
                            match &mut next.value {
                                UntaggedValue::Row(o) => {
                                    o.entries.insert(
                                        split_path[idx + 1].to_string(),
                                        new_value.value.clone().into_value(&self.tag),
                                    );
                                }
                                _ => {}
                            }

                            return Some(new_obj.clone());
                        } else {
                            match next.value {
                                UntaggedValue::Row(ref mut o) => {
                                    current = o;
                                }
                                _ => return None,
                            }
                        }
                    }
                    _ => return None,
                }
            }
        }

        None
    }

    pub fn insert_data_at_member(
        &mut self,
        member: &PathMember,
        new_value: Value,
    ) -> Result<(), ShellError> {
        match &mut self.value {
            UntaggedValue::Row(dict) => match &member.unspanned {
                UnspannedPathMember::String(key) => Ok({
                    dict.insert_data_at_key(key, new_value);
                }),
                UnspannedPathMember::Int(_) => Err(ShellError::type_error(
                    "column name",
                    "integer".spanned(member.span),
                )),
            },
            UntaggedValue::Table(array) => match &member.unspanned {
                UnspannedPathMember::String(_) => Err(ShellError::type_error(
                    "list index",
                    "string".spanned(member.span),
                )),
                UnspannedPathMember::Int(int) => Ok({
                    let int = int.to_usize().ok_or_else(|| {
                        ShellError::range_error(
                            ExpectedRange::Usize,
                            &"bigger number".spanned(member.span),
                            "inserting into a list",
                        )
                    })?;

                    insert_data_at_index(array, int.tagged(member.span), new_value.clone())?;
                }),
            },
            other => match &member.unspanned {
                UnspannedPathMember::String(_) => Err(ShellError::type_error(
                    "row",
                    other.type_name().spanned(self.span()),
                )),
                UnspannedPathMember::Int(_) => Err(ShellError::type_error(
                    "table",
                    other.type_name().spanned(self.span()),
                )),
            },
        }
    }

    pub fn insert_data_at_column_path(
        &self,
        split_path: &ColumnPath,
        new_value: Value,
    ) -> Result<Value, ShellError> {
        let (last, front) = split_path.split_last();
        let mut original = self.clone();

        let mut current: &mut Value = &mut original;

        for member in front {
            let type_name = current.spanned_type_name();

            current = current.get_mut_data_by_member(&member).ok_or_else(|| {
                ShellError::missing_property(
                    member.plain_string(std::usize::MAX).spanned(member.span),
                    type_name,
                )
            })?
        }

        current.insert_data_at_member(&last, new_value)?;

        Ok(original)
    }

    pub fn replace_data_at_column_path(
        &self,
        split_path: &ColumnPath,
        replaced_value: Value,
    ) -> Option<Value> {
        let mut new_obj: Value = self.clone();
        let mut current = &mut new_obj;
        let split_path = split_path.members();

        for idx in 0..split_path.len() {
            match current.get_mut_data_by_member(&split_path[idx]) {
                Some(next) => {
                    if idx == (split_path.len() - 1) {
                        *next = replaced_value.value.into_value(&self.tag);
                        return Some(new_obj);
                    } else {
                        current = next;
                    }
                }
                None => {
                    return None;
                }
            }
        }

        None
    }

    pub fn as_column_path(&self) -> Result<Tagged<ColumnPath>, ShellError> {
        match &self.value {
            UntaggedValue::Table(table) => {
                let mut out: Vec<PathMember> = vec![];

                for item in table {
                    out.push(item.as_path_member()?);
                }

                Ok(ColumnPath::new(out).tagged(&self.tag))
            }

            UntaggedValue::Primitive(Primitive::ColumnPath(path)) => {
                Ok(path.clone().tagged(self.tag.clone()))
            }

            other => Err(ShellError::type_error(
                "column path",
                other.type_name().spanned(self.span()),
            )),
        }
    }

    pub fn as_path_member(&self) -> Result<PathMember, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(primitive) => match primitive {
                Primitive::Int(int) => Ok(PathMember::int(int.clone(), self.tag.span)),
                Primitive::String(string) => Ok(PathMember::string(string, self.tag.span)),
                other => Err(ShellError::type_error(
                    "path member",
                    other.type_name().spanned(self.span()),
                )),
            },
            other => Err(ShellError::type_error(
                "path member",
                other.type_name().spanned(self.span()),
            )),
        }
    }

    pub fn as_string(&self) -> Result<String, ShellError> {
        match &self.value {
            UntaggedValue::Primitive(Primitive::String(s)) => Ok(s.clone()),
            UntaggedValue::Primitive(Primitive::Boolean(x)) => Ok(format!("{}", x)),
            UntaggedValue::Primitive(Primitive::Decimal(x)) => Ok(format!("{}", x)),
            UntaggedValue::Primitive(Primitive::Int(x)) => Ok(format!("{}", x)),
            UntaggedValue::Primitive(Primitive::Bytes(x)) => Ok(format!("{}", x)),
            UntaggedValue::Primitive(Primitive::Path(x)) => Ok(format!("{}", x.display())),
            // TODO: this should definitely be more general with better errors
            other => Err(ShellError::labeled_error(
                "Expected string",
                other.type_name(),
                &self.tag,
            )),
        }
    }
}

fn insert_data_at_index(
    list: &mut Vec<Value>,
    index: Tagged<usize>,
    new_value: Value,
) -> Result<(), ShellError> {
    if list.len() >= index.item {
        Err(ShellError::range_error(
            0..(list.len()),
            &format_args!("{}", index.item).spanned(index.tag.span),
            "insert at index",
        ))
    } else {
        list[index.item] = new_value;
        Ok(())
    }
}

impl Value {
    pub fn get_data(&self, desc: &String) -> MaybeOwned<'_, Value> {
        match &self.value {
            UntaggedValue::Primitive(_) => MaybeOwned::Borrowed(self),
            UntaggedValue::Row(o) => o.get_data(desc),
            UntaggedValue::Block(_) | UntaggedValue::Table(_) | UntaggedValue::Error(_) => {
                MaybeOwned::Owned(UntaggedValue::nothing().into_untagged_value())
            }
        }
    }

    pub(crate) fn get_data_by_index(&self, idx: Spanned<usize>) -> Option<Value> {
        match &self.value {
            UntaggedValue::Table(value_set) => {
                let value = value_set.get(idx.item)?;
                Some(
                    value
                        .value
                        .clone()
                        .into_value(Tag::new(value.anchor(), idx.span)),
                )
            }
            _ => None,
        }
    }

    pub(crate) fn get_data_by_key(&self, name: Spanned<&str>) -> Option<Value> {
        match &self.value {
            UntaggedValue::Row(o) => o.get_data_by_key(name),
            UntaggedValue::Table(l) => {
                let mut out = vec![];
                for item in l {
                    match item {
                        Value {
                            value: UntaggedValue::Row(o),
                            ..
                        } => match o.get_data_by_key(name) {
                            Some(v) => out.push(v),
                            None => out.push(UntaggedValue::nothing().into_untagged_value()),
                        },
                        _ => out.push(UntaggedValue::nothing().into_untagged_value()),
                    }
                }

                if out.len() > 0 {
                    Some(UntaggedValue::Table(out).into_value(name.span))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(crate) fn get_mut_data_by_member(&mut self, name: &PathMember) -> Option<&mut Value> {
        match &mut self.value {
            UntaggedValue::Row(o) => match &name.unspanned {
                UnspannedPathMember::String(string) => o.get_mut_data_by_key(&string),
                UnspannedPathMember::Int(_) => None,
            },
            UntaggedValue::Table(l) => match &name.unspanned {
                UnspannedPathMember::String(string) => {
                    for item in l {
                        match item {
                            Value {
                                value: UntaggedValue::Row(o),
                                ..
                            } => match o.get_mut_data_by_key(&string) {
                                Some(v) => return Some(v),
                                None => {}
                            },
                            _ => {}
                        }
                    }
                    None
                }
                UnspannedPathMember::Int(int) => {
                    let index = int.to_usize()?;
                    l.get_mut(index)
                }
            },
            _ => None,
        }
    }
}
