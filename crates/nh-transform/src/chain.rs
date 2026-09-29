//! Port of `transform/chain.go`.
//!
//! Owner: Wave B task T07 (transform-publisher).

//! Go `transform.Chain`: transformers applied in order, ping-ponging between two buffers.

use nh_common::Result;

/// Go: `transform.FromTo` — the input bytes and the output buffer of one step.
///
/// Go's `From()` is a `*bytes.Buffer` (a `BytesReader`); every transformer of the publish chain
/// reads it through `Bytes()` in one piece, so the port hands out the slice. `To()` is the other
/// pooled buffer, reset before the step.
pub struct FromTo<'a> {
    pub from: &'a [u8],
    pub to: &'a mut Vec<u8>,
}

/// Go: `transform.Transformer func(ft FromTo) error`.
pub type Transformer = Box<dyn Fn(&mut FromTo<'_>) -> Result<()> + Send + Sync>;

/// Go: `transform.Chain`.
#[derive(Default)]
pub struct Chain(pub Vec<Transformer>);

impl Chain {
    // Go: transform/chain.go:New
    pub fn new(trs: Vec<Transformer>) -> Self {
        Chain(trs)
    }

    /// Go: `NewEmpty()` (a chain with capacity 20).
    // Go: transform/chain.go:NewEmpty
    pub fn new_empty() -> Self {
        Chain(Vec::with_capacity(20))
    }

    /// Go: `append(transformers, t)`.
    pub fn push(&mut self, t: Transformer) {
        self.0.push(t);
    }

    /// Go: `len(c)`.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Go: `len(c) == 0`.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Go: `Chain.Apply(to, from)`: with no transformers `from` is copied to `to`.
    ///
    /// Go reads `from` completely into the first pooled buffer (`b1.ReadFrom`) before the first
    /// step, so how the source was chunked never reaches a transformer. The output of step `i` is
    /// the input of step `i+1`; the result is the output of the last step.
    // Go: transform/chain.go:Apply
    pub fn apply(&self, from: &[u8]) -> Result<Vec<u8>> {
        if self.0.is_empty() {
            return Ok(from.to_vec());
        }

        let mut b1: Vec<u8> = from.to_vec();
        let mut b2: Vec<u8> = Vec::new();

        // Go's fromToBuffer: `from` is b1 for the first step, then the buffers swap.
        let mut from_is_b1 = true;

        for (i, tr) in self.0.iter().enumerate() {
            if i > 0 {
                from_is_b1 = !from_is_b1;
                if from_is_b1 {
                    b2.clear();
                } else {
                    b1.clear();
                }
            }

            let res = if from_is_b1 {
                tr(&mut FromTo {
                    from: &b1,
                    to: &mut b2,
                })
            } else {
                tr(&mut FromTo {
                    from: &b2,
                    to: &mut b1,
                })
            };

            if let Err(err) = res {
                // Write output to a temp file so it can be read by the user for trouble shooting.
                let from = if from_is_b1 { &b1 } else { &b2 };
                let filename = match create_temp_with(from) {
                    Some(name) => name,
                    None => "output.html".to_string(),
                };
                return Err(nh_common::herrors::new_file_error_from_name(err, &filename));
            }
        }

        Ok(if from_is_b1 { b2 } else { b1 })
    }
}

/// Go's `os.CreateTemp("", "hugo-transform-error")` + `io.Copy(tempfile, fb.from)`: a new file
/// `hugo-transform-error<random decimal>` in `os.TempDir()` holding `content`; `None` when it
/// cannot be created (Go then names the error's file `output.html`).
fn create_temp_with(content: &[u8]) -> Option<String> {
    use std::io::Write;
    // Go: os.TempDir() ($TMPDIR, else /tmp).
    let dir = match std::env::var_os("TMPDIR") {
        Some(d) if !d.is_empty() => std::path::PathBuf::from(d),
        _ => std::path::PathBuf::from("/tmp"),
    };
    // Go: os.nextRandom (a random uint32 in decimal); try up to 10000 names like Go.
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        ^ (u64::from(std::process::id()) << 32);
    for _ in 0..10000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let name = dir.join(format!("hugo-transform-error{}", seed as u32));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&name)
        {
            Ok(mut f) => {
                let _ = f.write_all(content);
                return Some(name.to_string_lossy().into_owned());
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return None,
        }
    }
    None
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: transform/chain.go (124 lines; 4/5 funcs executed)
//   types: Transformer, BytesReader, FromTo, Chain, fromToBuffer
// OK L50-52: New(trs ...Transformer) Chain
// OK L55-57: NewEmpty() Chain
// OK L66-68: (ft fromToBuffer) From() BytesReader
// OK L70-72: (ft fromToBuffer) To() io.Writer
// OK L76-124: (c *Chain) Apply(to io.Writer, from io.Reader) error
// ---------------------------------------------------------------------------
