use std::fs::File;
use std::mem::size_of;
use std::path::PathBuf;

use csv::{ByteRecord, Reader, ReaderBuilder, Trim};
use sysinfo::System;

const MEBIBYTE: usize = 1024 * 1024;
const MINIMUM_MEMORY_BUDGET: usize = 8 * MEBIBYTE;
const FALLBACK_MEMORY_BUDGET: usize = 32 * MEBIBYTE;
const MAXIMUM_MEMORY_BUDGET: usize = 128 * MEBIBYTE;

/// Parsing settings resolved by the source before consumption begins.
#[derive(Clone, Debug)]
pub(crate) struct CsvConfiguration {
    pub path: PathBuf,
    pub delimiter: u8,
    pub quote: u8,
    pub escape: Option<u8>,
    pub header: bool,
    pub trim: bool,
}

/// Owned parsed records whose fields can be borrowed as byte slices.
///
/// Each record keeps the parser's byte offset, one-based line, and zero-based
/// record number in `ByteRecord::position()`. The header is record zero when present.
#[derive(Debug)]
pub(crate) struct CsvRecordBatch {
    pub records: Vec<ByteRecord>,
    /// Input bytes consumed by these records, including quoting and terminators.
    pub input_bytes: usize,
}

/// Lazily opens and incrementally reads a CSV file into bounded record batches.
pub(crate) struct CsvReader {
    configuration: CsvConfiguration,
    reader: Option<Reader<File>>,
    headers: Option<ByteRecord>,
    memory_budget_bytes: Option<usize>,
    finished: bool,
}

impl CsvReader {
    /// Construct a reader without opening the file or querying system memory.
    pub fn new(configuration: CsvConfiguration) -> Self {
        Self {
            configuration,
            reader: None,
            headers: None,
            memory_budget_bytes: None,
            finished: false,
        }
    }

    /// Return the parsed header after the first read call.
    pub fn headers(&self) -> Option<&ByteRecord> {
        self.headers.as_ref()
    }

    /// Report whether the first parsed record is treated as a header.
    pub fn header_enabled(&self) -> bool {
        self.configuration.header
    }

    /// Read exactly one record without decoding or collecting later records.
    ///
    /// A caller can stop after this record without parsing a following row.
    pub fn next_record(&mut self) -> Result<Option<ByteRecord>, csv::Error> {
        if self.finished {
            return Ok(None);
        }
        self.open_if_needed()?;
        let mut record = ByteRecord::new();
        if self
            .reader
            .as_mut()
            .expect("reader opened above")
            .read_byte_record(&mut record)?
        {
            Ok(Some(record))
        } else {
            self.finished = true;
            Ok(None)
        }
    }

    /// Read one batch, or `None` after the stream is exhausted.
    ///
    /// The budget is a soft bound: an individual record larger than it is
    /// returned alone so that large fields remain readable.
    pub fn next_batch(&mut self) -> Result<Option<CsvRecordBatch>, csv::Error> {
        if self.finished {
            return Ok(None);
        }
        self.open_if_needed()?;

        let budget = *self
            .memory_budget_bytes
            .get_or_insert_with(default_memory_budget);
        let reader = self.reader.as_mut().expect("reader opened above");
        let mut records = Vec::new();
        let mut input_bytes = 0usize;
        let mut estimated_memory_bytes = 0usize;

        // A batch ends on observed record storage, so wide records yield fewer
        // rows without guessing a fixed row count or changing the public API.
        while estimated_memory_bytes < budget {
            let mut record = ByteRecord::new();
            if !reader.read_byte_record(&mut record)? {
                self.finished = true;
                break;
            }
            let record_input_bytes = reader.position().byte().saturating_sub(
                record
                    .position()
                    .expect("parsed record has a position")
                    .byte(),
            );
            input_bytes = input_bytes.saturating_add(record_input_bytes as usize);
            estimated_memory_bytes = estimated_memory_bytes.saturating_add(
                record
                    .as_slice()
                    .len()
                    .saturating_add(record.len().saturating_mul(size_of::<usize>()))
                    .saturating_add(size_of::<ByteRecord>()),
            );
            records.push(record);
        }

        if records.is_empty() {
            return Ok(None);
        }
        Ok(Some(CsvRecordBatch {
            records,
            input_bytes,
        }))
    }

    /// Initialize the parser and header only on first consumption.
    fn open_if_needed(&mut self) -> Result<(), csv::Error> {
        if self.reader.is_some() {
            return Ok(());
        }
        let mut builder = ReaderBuilder::new();
        builder
            .delimiter(self.configuration.delimiter)
            .quote(self.configuration.quote)
            .escape(self.configuration.escape)
            .has_headers(self.configuration.header)
            .trim(if self.configuration.trim {
                Trim::All
            } else {
                Trim::None
            });
        let mut reader = builder.from_path(&self.configuration.path)?;
        if self.configuration.header {
            self.headers = Some(reader.byte_headers()?.clone());
        }
        self.reader = Some(reader);
        Ok(())
    }
}

/// Calculate the isolated batch budget from currently available RAM.
fn memory_budget(available_memory_bytes: Option<u64>) -> usize {
    available_memory_bytes
        .filter(|available| *available > 0)
        .map(|available| {
            (available / 100).clamp(MINIMUM_MEMORY_BUDGET as u64, MAXIMUM_MEMORY_BUDGET as u64)
                as usize
        })
        .unwrap_or(FALLBACK_MEMORY_BUDGET)
}

/// Sample available RAM once when the reader first needs a batch target.
fn default_memory_budget() -> usize {
    let mut system = System::new();
    system.refresh_memory();
    memory_budget(Some(system.available_memory()))
}

#[cfg(test)]
#[path = "reader.tests.rs"]
mod tests;
