use crate::rand::Rand;
use std::fs::OpenOptions;
use std::io::Write;

/// Parse byte count with optional K, M, G suffix
pub fn parse_byte_count(s: &str) -> Option<usize> {
    let s = s.trim().to_uppercase();
    let (num_str, multiplier) = if s.ends_with('K') {
        (&s[..s.len() - 1], 1024)
    } else if s.ends_with('M') {
        (&s[..s.len() - 1], 1024 * 1024)
    } else if s.ends_with('G') {
        (&s[..s.len() - 1], 1024 * 1024 * 1024)
    } else {
        (s.as_str(), 1)
    };
    num_str.parse::<usize>().ok().map(|n| n * multiplier)
}

fn write_bytes<W: Write>(out: &mut W, limit: Option<usize>) {
    let mut buf = [0u8; 65536];
    let mut written: usize = 0;

    loop {
        for chunk in buf.chunks_exact_mut(8) {
            chunk.copy_from_slice(&(Rand::get() as u64).to_le_bytes());
        }

        let to_write = if let Some(limit) = limit {
            let remaining = limit.saturating_sub(written);
            if remaining == 0 {
                break;
            }
            remaining.min(buf.len())
        } else {
            buf.len()
        };

        if out.write_all(&buf[..to_write]).is_err() {
            break;
        }
        written += to_write;

        if let Some(limit) = limit
            && written >= limit
        {
            break;
        }
    }
}

pub fn output(limit: Option<usize>, file_path: Option<&str>) {
    if let Some(path) = file_path {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .expect("Failed to open output file");
        write_bytes(&mut file, limit);
    } else {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        write_bytes(&mut out, limit);
    }
    crate::rand::shutdown_urandom();
}
