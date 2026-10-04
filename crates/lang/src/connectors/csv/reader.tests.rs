use super::*;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE_NUMBER: AtomicU64 = AtomicU64::new(0);

/// Own a uniquely named CSV file and remove it after a test finishes.
struct CsvFixture {
    path: PathBuf,
}

impl CsvFixture {
    /// Write the supplied bytes into a temporary fixture file.
    fn new(bytes: &[u8]) -> Self {
        let number = NEXT_FIXTURE_NUMBER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "eck-csv-reader-{}-{number}.csv",
            std::process::id()
        ));
        fs::write(&path, bytes).expect("write CSV fixture");
        Self { path }
    }

    /// Build the default reader configuration for this fixture.
    fn configuration(&self) -> CsvConfiguration {
        CsvConfiguration {
            path: self.path.clone(),
            delimiter: b',',
            quote: b'"',
            escape: None,
            header: false,
            trim: false,
        }
    }
}

impl Drop for CsvFixture {
    /// Remove the fixture even when a test assertion panics.
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Opening and missing-file errors occur only on the first read.
#[test]
fn opens_only_on_consumption() {
    let fixture = CsvFixture::new(b"first,second\n");
    let configuration = fixture.configuration();
    let reader = CsvReader::new(configuration.clone());
    assert!(reader.reader.is_none());
    assert!(reader.memory_budget_bytes.is_none());
    drop(reader);

    fs::remove_file(&fixture.path).expect("remove fixture before reading");
    let mut reader = CsvReader::new(configuration);
    assert!(reader.next_batch().is_err());
}

/// A row iterator sees the header and can stop before a malformed later row.
#[test]
fn reads_one_record_without_batch_prefetch() {
    let fixture = CsvFixture::new(b"name,number\nfirst,1\nbroken\n");
    let mut configuration = fixture.configuration();
    configuration.header = true;
    let mut reader = CsvReader::new(configuration);
    assert!(reader.header_enabled());
    assert!(reader.headers().is_none());

    let record = reader.next_record().unwrap().unwrap();
    assert!(reader.memory_budget_bytes.is_none());
    assert_eq!(reader.headers().unwrap().get(0), Some(&b"name"[..]));
    assert_eq!(record.get(0), Some(&b"first"[..]));
    assert_eq!(record.position().unwrap().record(), 1);
    drop(reader);

    let mut reader = CsvReader::new(fixture.configuration());
    assert!(reader.next_record().unwrap().is_some());
    assert!(reader.next_batch().is_err());
}

/// Row reads defer the RAM query until a caller first requests a bounded batch.
#[test]
fn initializes_memory_budget_only_for_batches() {
    let fixture = CsvFixture::new(b"one\ntwo\nthree\n");
    let mut reader = CsvReader::new(fixture.configuration());
    assert!(reader.next_record().unwrap().is_some());
    assert!(reader.memory_budget_bytes.is_none());
    let batch = reader.next_batch().unwrap().unwrap();
    assert_eq!(batch.records.len(), 2);
    let budget = reader.memory_budget_bytes.unwrap();
    assert!((MINIMUM_MEMORY_BUDGET..=MAXIMUM_MEMORY_BUDGET).contains(&budget));
    assert!(reader.next_batch().unwrap().is_none());
    assert_eq!(reader.memory_budget_bytes, Some(budget));
}

/// Quoting, mixed line endings, trimming, header positions, and raw bytes survive parsing.
#[test]
fn parses_headers_quotes_newlines_and_non_utf8_bytes() {
    let fixture = CsvFixture::new(b" left ; right\r\n 1 ;\"two\r\nlines\"\r\n 3 ; \xff\n");
    let mut configuration = fixture.configuration();
    configuration.delimiter = b';';
    configuration.header = true;
    configuration.trim = true;
    let mut reader = CsvReader::new(configuration);

    assert!(reader.headers().is_none());
    let batch = reader.next_batch().unwrap().unwrap();
    assert_eq!(reader.headers().unwrap().get(0), Some(&b"left"[..]));
    assert_eq!(reader.headers().unwrap().get(1), Some(&b"right"[..]));
    assert_eq!(batch.records.len(), 2);
    assert_eq!(batch.records[0].get(0), Some(&b"1"[..]));
    assert_eq!(batch.records[0].get(1), Some(&b"two\r\nlines"[..]));
    assert_eq!(batch.records[1].get(1), Some(&b"\xff"[..]));
    assert_eq!(batch.records[0].position().unwrap().record(), 1);
    assert_eq!(batch.records[0].position().unwrap().byte(), 14);
    assert_eq!(batch.records[1].position().unwrap().record(), 2);
    assert!(batch.input_bytes > 0);
    assert!(reader.next_batch().unwrap().is_none());
}

/// An explicit escape byte and doubled quotes both decode without field allocation.
#[test]
fn parses_escaped_quotes() {
    let fixture = CsvFixture::new(b"\"one \\\"quote\\\"\",\"two \"\"quotes\"\"\"\n");
    let mut configuration = fixture.configuration();
    configuration.escape = Some(b'\\');
    let mut reader = CsvReader::new(configuration);
    let batch = reader.next_batch().unwrap().unwrap();
    assert_eq!(batch.records[0].get(0), Some(&b"one \"quote\""[..]));
    assert_eq!(batch.records[0].get(1), Some(&b"two \"quotes\""[..]));
}

/// The final record is returned even without a terminating LF or CRLF.
#[test]
fn reads_final_record_without_newline() {
    let fixture = CsvFixture::new(b"id,name\r\n1,Ada\n2,Bob");
    let mut configuration = fixture.configuration();
    configuration.header = true;
    let mut reader = CsvReader::new(configuration);
    let batch = reader.next_batch().unwrap().unwrap();
    assert_eq!(batch.records.len(), 2);
    assert_eq!(batch.records[1].get(0), Some(&b"2"[..]));
    assert_eq!(batch.records[1].get(1), Some(&b"Bob"[..]));
    assert!(reader.next_batch().unwrap().is_none());
}

/// A record larger than the batch target still arrives intact as one batch.
#[test]
fn streams_oversized_record() {
    let mut contents = vec![b'a'; 9 * MEBIBYTE];
    contents.push(b'\n');
    contents.extend_from_slice(b"tail\n");
    let fixture = CsvFixture::new(&contents);
    let mut reader = CsvReader::new(fixture.configuration());
    reader.memory_budget_bytes = Some(MINIMUM_MEMORY_BUDGET);

    let first = reader.next_batch().unwrap().unwrap();
    assert_eq!(first.records.len(), 1);
    assert_eq!(first.records[0].get(0).unwrap().len(), 9 * MEBIBYTE);
    let second = reader.next_batch().unwrap().unwrap();
    assert_eq!(second.records[0].get(0), Some(&b"tail"[..]));
    assert!(reader.next_batch().unwrap().is_none());
}

/// Observed record storage, not a fixed row count, determines the batch boundary.
#[test]
fn adapts_batch_size_to_observed_bytes() {
    let fixture = CsvFixture::new(b"1234567890\n1234567890\n1234567890\n1234567890\n");
    let mut reader = CsvReader::new(fixture.configuration());
    reader.memory_budget_bytes = Some(size_of::<ByteRecord>() * 2 + 20);

    let first = reader.next_batch().unwrap().unwrap();
    assert_eq!(first.records.len(), 2);
    assert_eq!(first.input_bytes, 22);
    let second = reader.next_batch().unwrap().unwrap();
    assert_eq!(second.records.len(), 2);
    assert_eq!(second.records[0].position().unwrap().record(), 2);
}

/// Structural CSV errors retain the parser's row position and surface to the caller.
#[test]
fn reports_malformed_record() {
    let fixture = CsvFixture::new(b"one,two\nonly-one\n");
    let mut reader = CsvReader::new(fixture.configuration());
    let error = reader.next_batch().unwrap_err();
    assert_eq!(error.position().unwrap().line(), 2);
    assert_eq!(error.position().unwrap().record(), 1);
}

/// Dropping an unfinished stream releases its file handle immediately.
#[test]
fn drops_early_without_reading_remaining_records() {
    let fixture = CsvFixture::new(b"one\ntwo\nthree\n");
    let mut reader = CsvReader::new(fixture.configuration());
    reader.memory_budget_bytes = Some(1);
    assert_eq!(reader.next_batch().unwrap().unwrap().records.len(), 1);
    assert!(!reader.finished);
    drop(reader);
    fs::remove_file(&fixture.path).expect("remove CSV after early drop");
}

/// The policy clamps one percent of available RAM and falls back when unknown.
#[test]
fn clamps_memory_budget() {
    assert_eq!(memory_budget(None), FALLBACK_MEMORY_BUDGET);
    assert_eq!(memory_budget(Some(0)), FALLBACK_MEMORY_BUDGET);
    assert_eq!(
        memory_budget(Some(100 * MEBIBYTE as u64)),
        MINIMUM_MEMORY_BUDGET
    );
    assert_eq!(
        memory_budget(Some(3_200 * MEBIBYTE as u64)),
        FALLBACK_MEMORY_BUDGET
    );
    assert_eq!(
        memory_budget(Some(20_000 * MEBIBYTE as u64)),
        MAXIMUM_MEMORY_BUDGET
    );
}
