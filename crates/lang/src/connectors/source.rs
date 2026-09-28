//! Runtime boundary between lazy sources and ECK row iteration.

use std::sync::Arc;

use csv::ByteRecord;

use crate::RuntimeError;
use crate::semantic::{Registry, RowField, RowType, SemanticType, SourceType, TypeId, Value};

use super::csv::{CsvConfiguration, CsvReader};

/// A reusable CSV source description; cloning it never opens its file.
#[derive(Clone, Debug)]
pub(crate) struct CsvSource {
    pub configuration: CsvConfiguration,
    pub row_type: Option<Arc<RowType>>,
}

impl CsvSource {
    /// Wraps the source as a first-class lazy ECK value.
    pub fn into_value(self) -> Value {
        Value::new_source(
            SourceType {
                row: self.row_type.clone(),
            },
            self,
        )
    }

    /// Adds a logical row contract without opening or validating the file.
    pub fn with_row_type(mut self, row_type: Arc<RowType>) -> Self {
        self.row_type = Some(row_type);
        self
    }

    /// Constructs an unopened cursor for one traversal.
    pub fn cursor(&self) -> SourceCursor {
        SourceCursor::Csv(CsvCursor {
            reader: self.batch_cursor(),
            row_type: self.row_type.clone(),
            plans: None,
            row_number: 0,
            path: self.configuration.path.display().to_string(),
        })
    }

    /// Gives an internal batch consumer parsed byte records without row values.
    pub fn batch_cursor(&self) -> CsvReader {
        CsvReader::new(self.configuration.clone())
    }
}

/// One active cursor whose next row can fail only when reached.
pub(crate) enum SourceCursor {
    Csv(CsvCursor),
}

impl SourceCursor {
    /// Produces one owned semantic row and does no lookahead.
    pub fn next_value(&mut self, registry: &Registry) -> Result<Option<Value>, RuntimeError> {
        match self {
            Self::Csv(cursor) => cursor.next_value(registry),
        }
    }
}

/// One resolved field decoder and CSV column slot.
struct ColumnPlan {
    name: String,
    column: usize,
    expected_name: String,
    decoder: FieldDecoder,
}

/// The supported V1 CSV scalar conversion chosen once per field.
enum FieldDecoder {
    Text(TypeId),
    Numeric(TypeId),
    Boolean(TypeId),
    Nullable(Box<FieldDecoder>, TypeId),
}

/// A CSV cursor with a header-to-row decoding plan fixed at first consumption.
pub(crate) struct CsvCursor {
    reader: CsvReader,
    row_type: Option<Arc<RowType>>,
    plans: Option<Vec<ColumnPlan>>,
    row_number: u64,
    path: String,
}

impl CsvCursor {
    /// Reads and decodes only the requested record.
    fn next_value(&mut self, registry: &Registry) -> Result<Option<Value>, RuntimeError> {
        let Some(record) = self.reader.next_record().map_err(|error| {
            RuntimeError::Message(format!("CSV error in {}: {error}", self.path))
        })?
        else {
            if self.row_type.is_some() && self.plans.is_none() {
                self.initialize_plan(&ByteRecord::new(), registry)?;
            }
            return Ok(None);
        };
        self.row_number += 1;
        if self.plans.is_none() {
            self.initialize_plan(&record, registry)?;
        }
        let row_type = self
            .row_type
            .as_ref()
            .expect("first record installs a row type")
            .clone();
        let plans = self.plans.as_ref().expect("first record installs a plan");
        let mut values = Vec::with_capacity(plans.len());
        for plan in plans {
            let input = record
                .get(plan.column)
                .ok_or_else(|| self.type_error(plan, b"", "missing field"))?;
            let value = decode(input, &plan.decoder, registry)
                .map_err(|reason| self.type_error(plan, input, &reason))?;
            values.push(value);
        }
        Ok(Some(Value::new_row(row_type, values)))
    }

    /// Resolves column names and decoders once, after the header is available.
    fn initialize_plan(
        &mut self,
        record: &ByteRecord,
        registry: &Registry,
    ) -> Result<(), RuntimeError> {
        if self.row_type.is_none() {
            let fields = (0..record.len())
                .map(|index| {
                    let name = self
                        .reader
                        .headers()
                        .and_then(|headers| headers.get(index))
                        .and_then(|name| std::str::from_utf8(name).ok())
                        .map(str::to_string)
                        .unwrap_or_else(|| index.to_string());
                    RowField {
                        name,
                        semantic_type: SemanticType::Scalar(crate::semantic::ValueType::plain(
                            registry.default_string().expect("CSV requires string"),
                        )),
                        source_type_name: "string".into(),
                    }
                })
                .collect::<Vec<_>>();
            self.row_type = Some(Arc::new(RowType {
                fields: fields.into(),
            }));
        }
        let row_type = self.row_type.as_ref().expect("row type installed");
        let mut plans = Vec::with_capacity(row_type.fields.len());
        for (field_index, field) in row_type.fields.iter().enumerate() {
            let column = match self.reader.headers() {
                Some(headers) if self.reader.header_enabled() => headers
                    .iter()
                    .position(|header| header == field.name.as_bytes())
                    .ok_or_else(|| {
                        RuntimeError::Message(format!(
                            "CSV type error in {}: missing column {:?}",
                            self.path, field.name
                        ))
                    })?,
                _ => field_index,
            };
            plans.push(ColumnPlan {
                name: field.name.clone(),
                column,
                expected_name: field.source_type_name.clone(),
                decoder: select_decoder(&field.semantic_type, registry)?,
            });
        }
        self.plans = Some(plans);
        Ok(())
    }

    /// Builds a detailed diagnostic only on a failing record.
    fn type_error(&self, plan: &ColumnPlan, input: &[u8], reason: &str) -> RuntimeError {
        RuntimeError::Message(format!(
            "CSV type error in {} at row {}, column {:?} ({}): expected {}, found {:?}: {}",
            self.path,
            self.row_number,
            plan.name,
            plan.column,
            plan.expected_name,
            String::from_utf8_lossy(input),
            reason,
        ))
    }
}

/// Resolves a scalar or nullable scalar CSV field to one decoder.
fn select_decoder(
    semantic_type: &SemanticType,
    registry: &Registry,
) -> Result<FieldDecoder, RuntimeError> {
    if let SemanticType::Union(members) = semantic_type
        && members.len() == 2
        && let Some(non_null) = members.iter().find(|member| {
            !member.contains_scalar(crate::semantic::ValueType::plain(
                registry.default_null().expect("CSV requires null"),
            ))
        })
    {
        return Ok(FieldDecoder::Nullable(
            Box::new(select_decoder(non_null, registry)?),
            registry.default_null()?,
        ));
    }
    let SemanticType::Scalar(value_type) = semantic_type else {
        return Err(RuntimeError::Message(format!(
            "unsupported CSV field type {}",
            semantic_type_name(semantic_type, registry)
        )));
    };
    if value_type.subtype.is_some() {
        return Err(RuntimeError::Message(
            "CSV V1 does not decode qualified scalar fields".into(),
        ));
    }
    let descriptor = registry.type_descriptor(value_type.base)?;
    if value_type.base == registry.default_string()? {
        Ok(FieldDecoder::Text(value_type.base))
    } else if value_type.base == registry.default_boolean()? {
        Ok(FieldDecoder::Boolean(value_type.base))
    } else if descriptor.parse_numeric_literal.is_some() {
        Ok(FieldDecoder::Numeric(value_type.base))
    } else {
        Err(RuntimeError::Message(format!(
            "unsupported CSV field type {}",
            descriptor.name
        )))
    }
}

/// Converts one CSV byte slice directly into its resolved ECK scalar value.
fn decode(input: &[u8], decoder: &FieldDecoder, registry: &Registry) -> Result<Value, String> {
    match decoder {
        FieldDecoder::Nullable(_, null_type) if input.is_empty() => registry
            .parse_null("null", Some(*null_type))
            .map_err(|error| error.to_string()),
        FieldDecoder::Nullable(inner, _) => decode(input, inner, registry),
        FieldDecoder::Text(type_id) => {
            let text = std::str::from_utf8(input).map_err(|error| error.to_string())?;
            registry
                .parse_string(text, Some(*type_id))
                .map_err(|error| error.to_string())
        }
        FieldDecoder::Numeric(type_id) => {
            let text = std::str::from_utf8(input).map_err(|error| error.to_string())?;
            registry
                .parse_numeric(text, Some(*type_id))
                .map_err(|error| error.to_string())
        }
        FieldDecoder::Boolean(type_id) => {
            let text = std::str::from_utf8(input).map_err(|error| error.to_string())?;
            registry
                .parse_boolean(text, Some(*type_id))
                .map_err(|error| error.to_string())
        }
    }
}

/// Renders a compact expected type for CSV failures.
fn semantic_type_name(semantic_type: &SemanticType, registry: &Registry) -> String {
    match semantic_type {
        SemanticType::Scalar(value_type) => registry.value_type_name(*value_type),
        SemanticType::Union(members)
            if members.len() == 2
                && members.iter().any(|member| {
                    member.contains_scalar(crate::semantic::ValueType::plain(
                        registry.default_null().expect("CSV requires null"),
                    ))
                }) =>
        {
            let non_null = members
                .iter()
                .find(|member| {
                    !member.contains_scalar(crate::semantic::ValueType::plain(
                        registry.default_null().expect("CSV requires null"),
                    ))
                })
                .expect("two-member nullable union");
            format!("{}?", semantic_type_name(non_null, registry))
        }
        _ => format!("{semantic_type:?}"),
    }
}
