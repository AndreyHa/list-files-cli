use crate::clipboard::ClipboardSink;
use anyhow::Result;
use std::fs::File;
use std::io::{BufWriter, Write};

pub enum OutputSink<'a> {
    Clipboard(&'a dyn ClipboardSink),
    Stdout,
    File(BufWriter<File>),
}

impl<'a> OutputSink<'a> {
    pub fn write_all(&mut self, s: &str) -> Result<()> {
        match self {
            OutputSink::Stdout => {
                print!("{}", s);
                Ok(())
            }
            OutputSink::File(f) => {
                f.write_all(s.as_bytes())?;
                Ok(())
            }
            OutputSink::Clipboard(_) => {
                // For clipboard, we will buffer externally
                Ok(())
            }
        }
    }

    pub fn finish(self) -> Result<()> {
        match self {
            OutputSink::Stdout => Ok(()),
            OutputSink::File(mut f) => {
                f.flush()?;
                Ok(())
            }
            OutputSink::Clipboard(_) => {
                // Clipboard will be handled by BufferedOutputSink
                Ok(())
            }
        }
    }
}

pub struct BufferedOutputSink<'a> {
    sink: OutputSink<'a>,
    buffer: Option<String>,
}

impl<'a> BufferedOutputSink<'a> {
    pub fn new(sink: OutputSink<'a>) -> Self {
        let use_buffer = matches!(sink, OutputSink::Clipboard(_));
        BufferedOutputSink {
            sink,
            buffer: if use_buffer { Some(String::new()) } else { None },
        }
    }

    pub fn write_all(&mut self, s: &str) -> Result<()> {
        if let Some(ref mut buf) = self.buffer {
            buf.push_str(s);
        } else {
            self.sink.write_all(s)?;
        }
        Ok(())
    }

    pub fn finish(self) -> Result<()> {
        if let Some(content) = self.buffer {
            if let OutputSink::Clipboard(cb) = self.sink {
                cb.set_text(content).map_err(anyhow::Error::msg)?;
            }
        } else {
            self.sink.finish()?;
        }
        Ok(())
    }
}
