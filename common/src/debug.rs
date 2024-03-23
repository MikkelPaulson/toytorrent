use std::io;
use std::io::prelude::*;
use std::io::BufReader;

pub struct DebugBufReader<R: ?Sized> {
    prefix: String,
    inner: BufReader<R>,
}

pub struct DebugWriter<W: ?Sized> {
    prefix: String,
    inner: W,
}

impl<R: Read> DebugBufReader<R> {
    pub fn new(prefix: String, inner: R) -> Self {
        Self {
            prefix,
            inner: BufReader::new(inner),
        }
    }
}

impl<W: Write> DebugWriter<W> {
    pub fn new(prefix: String, inner: W) -> Self {
        Self { prefix, inner }
    }
}

impl<R: ?Sized + Read> Read for DebugBufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let result = self.inner.read(buf);
        match result {
            Ok(len) => {
                print_debug(&self.prefix, "<-", &buf[0..len]);
                Ok(len)
            }
            Err(e) => {
                print_error(&self.prefix, "<!", &e);
                Err(e)
            }
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let result = self.inner.read_exact(buf);
        match result {
            Ok(()) => {
                print_debug(&self.prefix, "<-", &buf);
                Ok(())
            }
            Err(e) => {
                print_error(&self.prefix, "<!", &e);
                Err(e)
            }
        }
    }
}

impl<R: ?Sized + Read> BufRead for DebugBufReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt)
    }
}

impl<W: ?Sized + Write> Write for DebugWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let result = self.inner.write(buf);
        match result {
            Ok(len) => {
                print_debug(&self.prefix, "->", &buf);
                Ok(len)
            }
            Err(e) => {
                print_error(&self.prefix, "!>", &e);
                Err(e)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn print_debug(prefix: &str, symbol: &str, data: &[u8]) {
    let print_debug_line = |prefix: &str, symbol: &str, data: &[u8]| {
        let mut output = String::with_capacity(82);

        output.push_str(prefix);
        (prefix.len()..=22).for_each(|_| output.push(' '));
        output.push_str(symbol);
        data.chunks_exact(2)
            .for_each(|chunk| output.push_str(&format!(" {:x}{:x}", chunk[0], chunk[1])));
        if data.len() % 2 == 1 {
            output.push_str(&format!(" {:x}  ", data.last().unwrap()));
        }
        (data.len().div_ceil(2)..=8)
            .chain(data.len()..=16)
            .chain(0..2)
            .for_each(|_| output.push(' '));
        data.iter().for_each(|i| {
            if i.is_ascii() && !i.is_ascii_control() {
                output.push(*i as char);
            } else {
                output.push('.');
            }
        });

        println!("{output}");
    };

    match data.len() {
        0..=16 => print_debug_line(prefix, symbol, data),
        17..=32 => {
            print_debug_line(prefix, symbol, &data[0..16]);
            print_debug_line(prefix, symbol, &data[16..]);
        }
        33.. => {
            print_debug_line(prefix, symbol, &data[0..16]);
            print_debug_line(prefix, symbol, &data[16..32]);
            println!(
                "{prefix:21} {symbol} ..... {} bytes total .....",
                data.len()
            );
        }
    }
}

fn print_error(prefix: &str, symbol: &str, error: &io::Error) {
    println!("{prefix:21} {symbol} {error:?}");
}
