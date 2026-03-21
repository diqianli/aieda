//! Output sink trait and implementations.
//!
//! This module defines the interface for output sinks that consume
//! simulation data and produce various output formats.

use std::io::{self, Write};

/// Trait for output sinks that consume simulation data
pub trait OutputSink {
    /// Write a line of output
    fn write_line(&mut self, line: &str) -> io::Result<()>;

    /// Flush any buffered output
    fn flush(&mut self) -> io::Result<()>;

    /// Get the number of lines written
    fn lines_written(&self) -> u64;

    /// Check if the sink is still valid for writing
    fn is_valid(&self) -> bool;
}

/// Output sink that writes to a file
pub struct FileSink {
    /// The file being written to
    file: std::fs::File,
    /// Number of lines written
    lines: u64,
}

impl FileSink {
    /// Create a new file sink
    pub fn new(path: &std::path::Path) -> io::Result<Self> {
        let file = std::fs::File::create(path)?;
        Ok(Self { file, lines: 0 })
    }

    /// Create a new file sink with options
    pub fn with_options(
        path: &std::path::Path,
        append: bool,
    ) -> io::Result<Self> {
        let file = if append {
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(path)?
        } else {
            std::fs::File::create(path)?
        };
        Ok(Self { file, lines: 0 })
    }
}

impl OutputSink for FileSink {
    fn write_line(&mut self, line: &str) -> io::Result<()> {
        writeln!(self.file, "{}", line)?;
        self.lines += 1;
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }

    fn lines_written(&self) -> u64 {
        self.lines
    }

    fn is_valid(&self) -> bool {
        true
    }
}

/// Output sink that writes to memory (for buffering)
pub struct MemorySink {
    /// Buffered lines
    buffer: Vec<String>,
    /// Maximum capacity (0 = unlimited)
    max_capacity: usize,
}

impl MemorySink {
    /// Create a new memory sink
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            max_capacity: 0,
        }
    }

    /// Create with a maximum capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            max_capacity: capacity,
        }
    }

    /// Get all buffered content as a single string
    pub fn to_string(&self) -> String {
        self.buffer.join("\n")
    }

    /// Get the buffered lines
    pub fn lines(&self) -> &[String] {
        &self.buffer
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Write the buffer to a file
    pub fn write_to_file(&self, path: &std::path::Path) -> io::Result<()> {
        let mut file = std::fs::File::create(path)?;
        for line in &self.buffer {
            writeln!(file, "{}", line)?;
        }
        Ok(())
    }
}

impl Default for MemorySink {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputSink for MemorySink {
    fn write_line(&mut self, line: &str) -> io::Result<()> {
        if self.max_capacity == 0 || self.buffer.len() < self.max_capacity {
            self.buffer.push(line.to_string());
        }
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn lines_written(&self) -> u64 {
        self.buffer.len() as u64
    }

    fn is_valid(&self) -> bool {
        self.max_capacity == 0 || self.buffer.len() < self.max_capacity
    }
}

/// Output sink that writes to stdout
pub struct StdoutSink {
    /// Number of lines written
    lines: u64,
}

impl StdoutSink {
    /// Create a new stdout sink
    pub fn new() -> Self {
        Self { lines: 0 }
    }
}

impl Default for StdoutSink {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputSink for StdoutSink {
    fn write_line(&mut self, line: &str) -> io::Result<()> {
        println!("{}", line);
        self.lines += 1;
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }

    fn lines_written(&self) -> u64 {
        self.lines
    }

    fn is_valid(&self) -> bool {
        true
    }
}

/// Null sink that discards all output
pub struct NullSink {
    lines: u64,
}

impl NullSink {
    /// Create a new null sink
    pub fn new() -> Self {
        Self { lines: 0 }
    }
}

impl Default for NullSink {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputSink for NullSink {
    fn write_line(&mut self, _line: &str) -> io::Result<()> {
        self.lines += 1;
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn lines_written(&self) -> u64 {
        self.lines
    }

    fn is_valid(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_memory_sink() {
        let mut sink = MemorySink::new();
        sink.write_line("line 1").unwrap();
        sink.write_line("line 2").unwrap();

        assert_eq!(sink.lines_written(), 2);
        assert_eq!(sink.to_string(), "line 1\nline 2");
    }

    #[test]
    fn test_memory_sink_capacity() {
        let mut sink = MemorySink::with_capacity(2);
        sink.write_line("line 1").unwrap();
        sink.write_line("line 2").unwrap();
        sink.write_line("line 3").unwrap(); // Should be dropped

        assert_eq!(sink.lines_written(), 2);
    }

    #[test]
    fn test_file_sink() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let mut sink = FileSink::new(path).unwrap();
        sink.write_line("line 1").unwrap();
        sink.write_line("line 2").unwrap();
        sink.flush().unwrap();

        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("line 1"));
        assert!(content.contains("line 2"));
    }

    #[test]
    fn test_null_sink() {
        let mut sink = NullSink::new();
        sink.write_line("line 1").unwrap();
        sink.write_line("line 2").unwrap();

        assert_eq!(sink.lines_written(), 2);
    }
}
