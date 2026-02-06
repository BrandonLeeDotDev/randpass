//! Password generation and output.

use std::io::Write;
use zeroize::Zeroize;

pub mod charset;
mod generate;
pub mod output;

pub use generate::generate;
pub use generate::generate_batch;
pub use generate::generate_from_charset;

/// Buffered writer that mlock's its buffer, zeroizes on every flush, and
/// munlock's + zeroizes on drop. Buffer never reallocates — writes that
/// would exceed capacity trigger a flush first.
pub(crate) struct SecureBufWriter<W: Write> {
    inner: W,
    buf: Vec<u8>,
}

impl<W: Write> SecureBufWriter<W> {
    pub fn new(inner: W) -> Self {
        let buf = Vec::with_capacity(8192);
        unsafe {
            libc::mlock(buf.as_ptr() as *const libc::c_void, buf.capacity());
        }
        Self { inner, buf }
    }
}

impl<W: Write> Write for SecureBufWriter<W> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if self.buf.len() + data.len() > self.buf.capacity() {
            self.flush()?;
        }
        if data.len() >= self.buf.capacity() {
            return self.inner.write(data);
        }
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if !self.buf.is_empty() {
            self.inner.write_all(&self.buf)?;
            self.buf.zeroize();
        }
        self.inner.flush()
    }
}

impl<W: Write> Drop for SecureBufWriter<W> {
    fn drop(&mut self) {
        let _ = self.flush();
        let ptr = self.buf.as_ptr();
        let cap = self.buf.capacity();
        self.buf.zeroize();
        unsafe {
            libc::munlock(ptr as *const libc::c_void, cap);
        }
    }
}
