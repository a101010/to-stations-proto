//! The Send tab's form model and typed writer dispatch.
//!
//! A form is generated from a topic's [`DdsFieldMetadata`] schema rather than
//! from hand-written UI. The current registry only needs strings, numbers,
//! bools, enums, bitmasks, and nested structs; other schema kinds are shown as
//! unsupported and block sending with a clear error.

use std::collections::BTreeMap;

use stations_dds::cyclonedds::{
    DataWriter, DdsFieldMetadata, DynamicPrimitiveKind, DynamicTypeSchema, DynamicValue,
};
use stations_dds::dds::Node;
use stations_dds::hello_world::{HelloWorldTopic, HelloWorldViewTopic};

/// How a field is edited in the Send form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    String,
    Bool,
    Int(DynamicPrimitiveKind),
    Float(DynamicPrimitiveKind),
    Enum,
    Bitmask,
    Group,
    Unsupported,
}

impl FieldKind {
    pub fn as_str(self) -> &'static str {
        match self {
            FieldKind::String => "string",
            FieldKind::Bool => "bool",
            FieldKind::Int(_) => "int",
            FieldKind::Float(_) => "float",
            FieldKind::Enum => "enum",
            FieldKind::Bitmask => "bitmask",
            FieldKind::Group => "group",
            FieldKind::Unsupported => "unsupported",
        }
    }
}

/// One row in the generated form. Group rows label a nested struct; the rest
/// hold an initial value that the UI displays.
#[derive(Debug, Clone, PartialEq)]
pub struct FormRow {
    pub path: String,
    pub label: String,
    pub kind: FieldKind,
    pub indent: u32,
    pub value: String,
    pub bool_value: bool,
    pub unsupported: bool,
}

impl FormRow {
    pub fn is_group(&self) -> bool {
        self.kind == FieldKind::Group
    }
}

/// A schema-generated form. Edits are kept here, not in the Slint row model, so
/// typing does not disturb the input cursor.
pub struct SendForm {
    idl_type: String,
    schema: &'static DynamicTypeSchema,
    rows: Vec<FormRow>,
    values: BTreeMap<String, String>,
    bools: BTreeMap<String, bool>,
}

impl SendForm {
    pub fn from_schema(idl_type: &str, schema: &'static DynamicTypeSchema) -> Self {
        let defaults = schema.default_value();
        let mut rows = Vec::new();
        flatten(&defaults, schema, "", 0, &mut rows);
        Self {
            idl_type: idl_type.to_string(),
            schema,
            rows,
            values: BTreeMap::new(),
            bools: BTreeMap::new(),
        }
    }

    pub fn rows(&self) -> &[FormRow] {
        &self.rows
    }

    pub fn has_unsupported(&self) -> bool {
        self.rows.iter().any(|row| row.unsupported)
    }

    pub fn set_text(&mut self, path: &str, text: impl Into<String>) {
        self.values.insert(path.to_string(), text.into());
    }

    pub fn set_bool(&mut self, path: &str, value: bool) {
        self.bools.insert(path.to_string(), value);
    }

    /// Build a dynamic value from the current form, validating each field and
    /// the completed tree against the generated schema.
    pub fn to_dynamic_value(&self) -> Result<DynamicValue, String> {
        let mut tree = self.schema.default_value();
        for row in &self.rows {
            match row.kind {
                FieldKind::Group => {}
                FieldKind::Unsupported => {
                    return Err(format!(
                        "field `{}` has an unsupported type for sending",
                        row.path
                    ))
                }
                FieldKind::Bool => {
                    let value = self.bools.get(&row.path).copied().unwrap_or(row.bool_value);
                    set_path(&mut tree, self.schema, &row.path, DynamicValue::Bool(value))?;
                }
                FieldKind::String => {
                    let value = self.text_value(row);
                    set_path(
                        &mut tree,
                        self.schema,
                        &row.path,
                        DynamicValue::String(value.to_string()),
                    )?;
                }
                FieldKind::Int(kind) => {
                    let value = parse_int(kind, &row.path, self.text_value(row))?;
                    set_path(&mut tree, self.schema, &row.path, value)?;
                }
                FieldKind::Float(kind) => {
                    let value = parse_float(kind, &row.path, self.text_value(row))?;
                    set_path(&mut tree, self.schema, &row.path, value)?;
                }
                FieldKind::Enum => {
                    let value = self
                        .text_value(row)
                        .parse::<i32>()
                        .map_err(|e| format!("field `{}`: {e}", row.path))?;
                    set_path(
                        &mut tree,
                        self.schema,
                        &row.path,
                        DynamicValue::Enum { value },
                    )?;
                }
                FieldKind::Bitmask => {
                    let value = self
                        .text_value(row)
                        .parse::<u64>()
                        .map_err(|e| format!("field `{}`: {e}", row.path))?;
                    set_path(
                        &mut tree,
                        self.schema,
                        &row.path,
                        DynamicValue::Bitmask(value),
                    )?;
                }
            }
        }
        tree.validate_against(self.schema)
            .map_err(|e| format!("invalid {}: {e}", self.idl_type))?;
        Ok(tree)
    }

    fn text_value<'a>(&'a self, row: &'a FormRow) -> &'a str {
        self.values
            .get(&row.path)
            .map(String::as_str)
            .unwrap_or(&row.value)
    }
}

/// The generated schema for a registered DDS type.
pub fn schema_for(idl_type: &str) -> Option<&'static DynamicTypeSchema> {
    match idl_type {
        "hello_world::HelloWorldTopic" => {
            Some(<HelloWorldTopic as DdsFieldMetadata>::field_schema())
        }
        "hello_world::HelloWorldViewTopic" => {
            Some(<HelloWorldViewTopic as DdsFieldMetadata>::field_schema())
        }
        _ => None,
    }
}

/// A typed writer for one registered topic.
pub enum TopicWriter {
    HelloWorld(DataWriter<HelloWorldTopic>),
    HelloWorldView(DataWriter<HelloWorldViewTopic>),
}

impl TopicWriter {
    pub fn new(node: &Node, topic: &str, idl_type: &str) -> Result<Self, String> {
        match idl_type {
            "hello_world::HelloWorldTopic" => node
                .writer::<HelloWorldTopic>(topic)
                .map(TopicWriter::HelloWorld)
                .map_err(|e| e.to_string()),
            "hello_world::HelloWorldViewTopic" => node
                .writer::<HelloWorldViewTopic>(topic)
                .map(TopicWriter::HelloWorldView)
                .map_err(|e| e.to_string()),
            other => Err(format!("unsupported topic type `{other}`")),
        }
    }

    /// The number of matched DDS readers for this writer.
    pub fn matched_readers(&self) -> Result<usize, String> {
        match self {
            TopicWriter::HelloWorld(writer) => writer
                .matched_subscriptions()
                .map(|handles| handles.len())
                .map_err(|e| e.to_string()),
            TopicWriter::HelloWorldView(writer) => writer
                .matched_subscriptions()
                .map(|handles| handles.len())
                .map_err(|e| e.to_string()),
        }
    }

    pub fn write_dynamic(&self, value: &DynamicValue) -> Result<(), String> {
        match self {
            TopicWriter::HelloWorld(writer) => {
                let sample = HelloWorldTopic::from_dynamic_value(value)
                    .map_err(|e| format!("HelloWorldTopic: {e}"))?;
                writer.write(&sample).map_err(|e| e.to_string())
            }
            TopicWriter::HelloWorldView(writer) => {
                let sample = HelloWorldViewTopic::from_dynamic_value(value)
                    .map_err(|e| format!("HelloWorldViewTopic: {e}"))?;
                writer.write(&sample).map_err(|e| e.to_string())
            }
        }
    }
}

fn flatten(
    defaults: &DynamicValue,
    schema: &DynamicTypeSchema,
    prefix: &str,
    indent: u32,
    rows: &mut Vec<FormRow>,
) {
    let schema = effective(schema);
    let DynamicTypeSchema::Struct { fields, .. } = schema else {
        return;
    };
    for field in fields {
        let path = if prefix.is_empty() {
            field.name.clone()
        } else {
            format!("{prefix}.{}", field.name)
        };
        let value = defaults.field(&field.name);
        let kind = classify(field);
        if kind == FieldKind::Group {
            rows.push(FormRow {
                path: path.clone(),
                label: field.name.clone(),
                kind,
                indent,
                value: String::new(),
                bool_value: false,
                unsupported: false,
            });
            if let Some(child) = value {
                flatten(child, &field.value, &path, indent + 1, rows);
            }
            continue;
        }
        let (text, bool_value) = match value {
            Some(value) => format_value(kind, value),
            None => (String::new(), false),
        };
        let unsupported = kind == FieldKind::Unsupported;
        rows.push(FormRow {
            path,
            label: field.name.clone(),
            kind,
            indent,
            value: if unsupported {
                unsupported_reason(&field.value).to_string()
            } else {
                text
            },
            bool_value,
            unsupported,
        });
    }
}

fn classify(field: &stations_dds::cyclonedds::DynamicFieldSchema) -> FieldKind {
    if field.optional {
        return FieldKind::Unsupported;
    }
    classify_schema(&field.value)
}

fn classify_schema(schema: &DynamicTypeSchema) -> FieldKind {
    match effective(schema) {
        DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Boolean) => FieldKind::Bool,
        DynamicTypeSchema::Primitive(
            kind @ (DynamicPrimitiveKind::Float32 | DynamicPrimitiveKind::Float64),
        ) => FieldKind::Float(*kind),
        DynamicTypeSchema::Primitive(kind) => FieldKind::Int(*kind),
        DynamicTypeSchema::String { .. } => FieldKind::String,
        DynamicTypeSchema::Enum { .. } => FieldKind::Enum,
        DynamicTypeSchema::Bitmask { .. } => FieldKind::Bitmask,
        DynamicTypeSchema::Struct { .. } => FieldKind::Group,
        _ => FieldKind::Unsupported,
    }
}

fn unsupported_reason(schema: &DynamicTypeSchema) -> &'static str {
    match effective(schema) {
        DynamicTypeSchema::Sequence { .. } => "unsupported sequence",
        DynamicTypeSchema::Array { .. } => "unsupported array",
        DynamicTypeSchema::Union { .. } => "unsupported union",
        DynamicTypeSchema::Map { .. } => "unsupported map",
        DynamicTypeSchema::Alias { .. } => "unsupported alias",
        _ => "unsupported type",
    }
}

fn effective(schema: &DynamicTypeSchema) -> &DynamicTypeSchema {
    match schema {
        DynamicTypeSchema::Alias { target, .. } => effective(target),
        other => other,
    }
}

fn format_value(kind: FieldKind, value: &DynamicValue) -> (String, bool) {
    match (kind, value) {
        (FieldKind::Bool, DynamicValue::Bool(value)) => (String::new(), *value),
        (FieldKind::String, DynamicValue::String(value)) => (value.clone(), false),
        (FieldKind::Int(_), value) => (primitive_text(value), false),
        (FieldKind::Float(_), value) => (primitive_text(value), false),
        (FieldKind::Enum, DynamicValue::Enum { value }) => (value.to_string(), false),
        (FieldKind::Bitmask, DynamicValue::Bitmask(value)) => (value.to_string(), false),
        _ => (String::new(), false),
    }
}

fn primitive_text(value: &DynamicValue) -> String {
    match value {
        DynamicValue::Byte(v) => v.to_string(),
        DynamicValue::Int8(v) => v.to_string(),
        DynamicValue::UInt8(v) => v.to_string(),
        DynamicValue::Int16(v) => v.to_string(),
        DynamicValue::UInt16(v) => v.to_string(),
        DynamicValue::Int32(v) => v.to_string(),
        DynamicValue::UInt32(v) => v.to_string(),
        DynamicValue::Int64(v) => v.to_string(),
        DynamicValue::UInt64(v) => v.to_string(),
        DynamicValue::Float32(v) => v.to_string(),
        DynamicValue::Float64(v) => v.to_string(),
        DynamicValue::Char8(v) => v.to_string(),
        DynamicValue::Char16(v) => v.to_string(),
        other => format!("{other:?}"),
    }
}

fn parse_int(kind: DynamicPrimitiveKind, path: &str, text: &str) -> Result<DynamicValue, String> {
    let error = |e: std::num::ParseIntError| format!("field `{path}`: {e}");
    Ok(match kind {
        DynamicPrimitiveKind::Byte => DynamicValue::Byte(text.parse().map_err(error)?),
        DynamicPrimitiveKind::Int8 => DynamicValue::Int8(text.parse().map_err(error)?),
        DynamicPrimitiveKind::UInt8 => DynamicValue::UInt8(text.parse().map_err(error)?),
        DynamicPrimitiveKind::Int16 => DynamicValue::Int16(text.parse().map_err(error)?),
        DynamicPrimitiveKind::UInt16 => DynamicValue::UInt16(text.parse().map_err(error)?),
        DynamicPrimitiveKind::Int32 => DynamicValue::Int32(text.parse().map_err(error)?),
        DynamicPrimitiveKind::UInt32 => DynamicValue::UInt32(text.parse().map_err(error)?),
        DynamicPrimitiveKind::Int64 => DynamicValue::Int64(text.parse().map_err(error)?),
        DynamicPrimitiveKind::UInt64 => DynamicValue::UInt64(text.parse().map_err(error)?),
        DynamicPrimitiveKind::Char8 => DynamicValue::Char8(text.parse().map_err(error)?),
        DynamicPrimitiveKind::Char16 => DynamicValue::Char16(text.parse().map_err(error)?),
        other => return Err(format!("field `{path}`: unexpected integer kind {other:?}")),
    })
}

fn parse_float(kind: DynamicPrimitiveKind, path: &str, text: &str) -> Result<DynamicValue, String> {
    let error = |e: std::num::ParseFloatError| format!("field `{path}`: {e}");
    Ok(match kind {
        DynamicPrimitiveKind::Float32 => DynamicValue::Float32(text.parse().map_err(error)?),
        DynamicPrimitiveKind::Float64 => DynamicValue::Float64(text.parse().map_err(error)?),
        other => return Err(format!("field `{path}`: unexpected float kind {other:?}")),
    })
}

fn set_path(
    tree: &mut DynamicValue,
    schema: &DynamicTypeSchema,
    path: &str,
    value: DynamicValue,
) -> Result<(), String> {
    let parts: Vec<&str> = path.split('.').collect();
    set_path_parts(tree, schema, &parts, value)
}

fn set_path_parts(
    tree: &mut DynamicValue,
    schema: &DynamicTypeSchema,
    parts: &[&str],
    value: DynamicValue,
) -> Result<(), String> {
    let schema = effective(schema);
    let (name, rest) = parts
        .split_first()
        .ok_or_else(|| "empty field path".to_string())?;
    let field = schema
        .field(name)
        .ok_or_else(|| format!("field `{name}` not present in schema"))?;
    if rest.is_empty() {
        return tree
            .set_field(schema, name, value)
            .map_err(|e| format!("field `{name}`: {e}"));
    }
    let fields = tree
        .as_struct_mut()
        .ok_or_else(|| format!("field `{name}` is not a struct"))?;
    let child = fields
        .get_mut(*name)
        .ok_or_else(|| format!("field `{name}` is missing from the default value"))?;
    set_path_parts(child, &field.value, rest, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    static DDS_LOCK: Mutex<()> = Mutex::new(());

    fn dds_guard() -> std::sync::MutexGuard<'static, ()> {
        DDS_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("REPO_ROOT"))
    }

    fn hello_form() -> SendForm {
        let schema = schema_for("hello_world::HelloWorldTopic").unwrap();
        SendForm::from_schema("hello_world::HelloWorldTopic", schema)
    }

    fn view_form() -> SendForm {
        let schema = schema_for("hello_world::HelloWorldViewTopic").unwrap();
        SendForm::from_schema("hello_world::HelloWorldViewTopic", schema)
    }

    #[test]
    fn enumerates_nested_fields() {
        let form = hello_form();
        let paths: Vec<&str> = form.rows().iter().map(|row| row.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "greeting",
                "attitude",
                "attitude.x",
                "attitude.y",
                "attitude.z",
                "attitude.w",
            ]
        );
        assert!(form.rows()[1].is_group());
    }

    #[test]
    fn default_form_maps_to_typed_sample() {
        let dynamic = hello_form().to_dynamic_value().unwrap();
        let sample = HelloWorldTopic::from_dynamic_value(&dynamic).unwrap();
        assert_eq!(sample, HelloWorldTopic::default());
    }

    #[test]
    fn edited_values_survive_typed_round_trip() {
        let mut form = hello_form();
        form.set_text("greeting", "dds-echo-send-test");
        form.set_text("attitude.x", "0.25");
        form.set_text("attitude.y", "0.5");
        form.set_text("attitude.z", "0.75");
        form.set_text("attitude.w", "1");

        let dynamic = form.to_dynamic_value().unwrap();
        let sample = HelloWorldTopic::from_dynamic_value(&dynamic).unwrap();
        assert_eq!(sample.greeting, "dds-echo-send-test");
        assert_eq!(sample.attitude.x, 0.25);
        assert_eq!(sample.attitude.y, 0.5);
        assert_eq!(sample.attitude.z, 0.75);
        assert_eq!(sample.attitude.w, 1.0);
    }

    #[test]
    fn bool_field_maps_to_view_topic() {
        let mut form = view_form();
        form.set_text("greeting", "view send");
        form.set_text("attitude.w", "1");
        form.set_bool("valid", true);

        let dynamic = form.to_dynamic_value().unwrap();
        let sample = HelloWorldViewTopic::from_dynamic_value(&dynamic).unwrap();
        assert_eq!(sample.greeting, "view send");
        assert_eq!(sample.attitude.w, 1.0);
        assert!(sample.valid);
    }

    #[test]
    fn invalid_number_reports_field_path() {
        let mut form = hello_form();
        form.set_text("attitude.x", "abc");
        let error = form.to_dynamic_value().unwrap_err();
        assert!(error.contains("attitude.x"), "{error}");
    }

    #[test]
    fn every_registered_topic_has_a_sender() {
        let registry =
            stations_dds::contract::TopicRegistry::load(&repo_root().join("contracts/topics.toml"))
                .unwrap();
        for topic in &registry.topics {
            assert!(
                schema_for(&topic.idl_type).is_some(),
                "no send handler for registered type `{}`",
                topic.idl_type
            );
        }
    }

    #[test]
    fn sends_a_form_through_the_typed_writer() {
        let _guard = dds_guard();
        std::env::set_var(
            "CYCLONEDDS_URI",
            format!("file://{}/DDS/cyclonedds-config.xml", repo_root().display()),
        );
        let registry =
            stations_dds::contract::TopicRegistry::load(&repo_root().join("contracts/topics.toml"))
                .unwrap();
        let node = Node::new(0, registry).unwrap();
        let writer =
            TopicWriter::new(&node, "HelloWorldTopic", "hello_world::HelloWorldTopic").unwrap();
        let reader = node.reader::<HelloWorldTopic>("HelloWorldTopic").unwrap();

        let mut form = hello_form();
        form.set_text("greeting", "dds-echo-send-test");
        form.set_text("attitude.w", "1");
        let value = form.to_dynamic_value().unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            writer.write_dynamic(&value).unwrap();
            if let Some(sample) = reader.take().unwrap().into_iter().next() {
                assert_eq!(sample.greeting, "dds-echo-send-test");
                assert_eq!(sample.attitude.w, 1.0);
                return;
            }
            assert!(Instant::now() < deadline, "no sample received");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn matched_readers_tracks_a_subscriber() {
        let _guard = dds_guard();
        std::env::set_var(
            "CYCLONEDDS_URI",
            format!("file://{}/DDS/cyclonedds-config.xml", repo_root().display()),
        );
        let registry =
            stations_dds::contract::TopicRegistry::load(&repo_root().join("contracts/topics.toml"))
                .unwrap();
        let node = Node::new(0, registry).unwrap();
        let writer =
            TopicWriter::new(&node, "HelloWorldTopic", "hello_world::HelloWorldTopic").unwrap();
        assert_eq!(writer.matched_readers().unwrap(), 0);

        let _reader = node.reader::<HelloWorldTopic>("HelloWorldTopic").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if writer.matched_readers().unwrap() > 0 {
                return;
            }
            assert!(Instant::now() < deadline, "writer did not match the reader");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
