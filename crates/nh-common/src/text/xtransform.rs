//! Port of `golang.org/x/text/transform` (x/text v0.26.0): the `Transformer` interface, `Chain`,
//! `String`, `Bytes` and `RemoveFunc`, as used by `common/text.RemoveAccents`.
//!
//! Not ported (no caller): `Reader`, `Writer`, `Append`, `Nop`, `Discard` and
//! `SpanningTransformer`. Go's error values become [`TransformError`]; a `(nDst, nSrc, err)`
//! result is a tuple with `Option<TransformError>`.

use std::fmt;

use go_unicode::utf8;

/// The errors of x/text's transform package.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformError {
    /// Go: `ErrShortDst` — the destination buffer was too short to receive all of the
    /// transformed bytes.
    ShortDst,
    /// Go: `ErrShortSrc` — the source buffer has insufficient data to complete the
    /// transformation.
    ShortSrc,
    /// Go: `ErrEndOfSpan` — the input and output (the transformed input) are not identical.
    EndOfSpan,
    /// Go: `errInconsistentByteCount`.
    InconsistentByteCount,
    /// Go: `errShortInternal` — an internal buffer is not large enough to make progress.
    ShortInternal,
}

impl fmt::Display for TransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TransformError::ShortDst => "transform: short destination buffer",
            TransformError::ShortSrc => "transform: short source buffer",
            TransformError::EndOfSpan => "transform: input and output are not identical",
            TransformError::InconsistentByteCount => "transform: inconsistent byte count returned",
            TransformError::ShortInternal => "transform: short internal buffer",
        })
    }
}

impl std::error::Error for TransformError {}

/// Go: `transform.Transformer`.
pub trait Transformer {
    /// Go: `Transform(dst, src []byte, atEOF bool) (nDst, nSrc int, err error)`.
    fn transform(
        &mut self,
        dst: &mut [u8],
        src: &[u8],
        at_eof: bool,
    ) -> (usize, usize, Option<TransformError>);

    /// Go: `Reset()`.
    fn reset(&mut self);
}

/// Go: `transform.defaultBufSize`.
const DEFAULT_BUF_SIZE: usize = 4096;

/// Go: `transform.link`. The first link's source and the last link's destination are the
/// caller's buffers, passed to every `Transform` call; only `p` and `n` are kept for them.
struct Link {
    t: Option<Box<dyn Transformer>>,
    /// b[p:n] holds the bytes to be transformed by t.
    b: Vec<u8>,
    p: usize,
    n: usize,
}

/// Go: `transform.chain` (returned by [`chain`]).
pub struct Chain {
    link: Vec<Link>,
    err: Option<TransformError>,
    /// errStart is the index at which the error occurred plus 1. Processing
    /// errStart at this level at the next call to Transform. As long as
    /// errStart > 0, chain will not consume any more source bytes.
    err_start: usize,
}

impl Chain {
    // Go: transform.go:chain.fatalError
    fn fatal_error(&mut self, err_index: usize, err: TransformError) {
        let i = err_index + 1;
        if i > self.err_start {
            self.err_start = i;
            self.err = Some(err);
        }
    }
}

/// Go: `transform.Chain(t...)` — a Transformer that applies t in sequence. (An empty chain is
/// Go's `nop{}`, which is not ported; it is a chain without links here.)
// Go: transform.go:Chain
pub fn chain(t: Vec<Box<dyn Transformer>>) -> Chain {
    let n = t.len();
    let mut link: Vec<Link> = (0..=n)
        .map(|_| Link {
            t: None,
            b: Vec::new(),
            p: 0,
            n: 0,
        })
        .collect();
    for (i, tt) in t.into_iter().enumerate() {
        link[i].t = Some(tt);
    }
    // Use arrays to avoid allocations.
    for l in link.iter_mut().take(n).skip(1) {
        l.b = vec![0; DEFAULT_BUF_SIZE];
    }
    Chain {
        link,
        err: None,
        err_start: 0,
    }
}

impl Transformer for Chain {
    /// Go: `chain.Reset`.
    // Go: transform.go:chain.Reset
    fn reset(&mut self) {
        for l in self.link.iter_mut() {
            if let Some(t) = l.t.as_mut() {
                t.reset();
            }
            l.p = 0;
            l.n = 0;
        }
    }

    /// Go: `chain.Transform` — applies the transformers of c in sequence.
    // Go: transform.go:chain.Transform
    fn transform(
        &mut self,
        dst: &mut [u8],
        src: &[u8],
        at_eof: bool,
    ) -> (usize, usize, Option<TransformError>) {
        let last = self.link.len() - 1;
        if last == 0 {
            // Go: nop.Transform
            let n = dst.len().min(src.len());
            dst[..n].copy_from_slice(&src[..n]);
            let err = (n < src.len()).then_some(TransformError::ShortDst);
            return (n, n, err);
        }
        // Set up src and dst in the chain.
        self.link[0].p = 0;
        self.link[0].n = src.len();
        self.link[last].n = 0;
        let mut err = None;
        let mut last_full = false;
        let mut need_progress;

        // Loop over all transformers, feeding the output of one transformer to the next.
        let mut low = self.err_start;
        let mut i = self.err_start;
        let high = last - 1;
        while low <= i && i <= high {
            let (n_src, err0, in_len) = {
                let (left, right) = self.link.split_at_mut(i + 1);
                let in_l = &mut left[i];
                let out_l = &mut right[0];
                let in_src: &[u8] = if i == 0 {
                    &src[in_l.p..in_l.n]
                } else {
                    &in_l.b[in_l.p..in_l.n]
                };
                let out_dst: &mut [u8] = if i + 1 == last {
                    &mut dst[out_l.n..]
                } else {
                    &mut out_l.b[out_l.n..]
                };
                let t = in_l.t.as_mut().expect("chain link without transformer");
                let (n_dst, n_src, err0) = t.transform(out_dst, in_src, at_eof && low == i);
                out_l.n += n_dst;
                in_l.p += n_src;
                if i > 0 && in_l.p == in_l.n {
                    in_l.p = 0;
                    in_l.n = 0;
                }
                (n_src, err0, in_l.b.len())
            };
            need_progress = last_full;
            last_full = false;
            match err0 {
                Some(TransformError::ShortDst) => {
                    // Process the destination buffer next. Return if we are already
                    // at the high index.
                    if i == high {
                        return (
                            self.link[last].n,
                            self.link[0].p,
                            Some(TransformError::ShortDst),
                        );
                    }
                    if self.link[i + 1].n != 0 {
                        i += 1;
                        // If the Transformer at the next index is not able to process any
                        // source bytes there is nothing that can be done to make progress
                        // and the bytes will remain unprocessed. lastFull is used to
                        // detect this and break out of the loop with a fatal error.
                        last_full = true;
                        continue;
                    }
                    // The destination buffer was too small, but is completely empty.
                    // Return a fatal error as this transformation can never complete.
                    self.fatal_error(i, TransformError::ShortInternal);
                }
                Some(TransformError::ShortSrc) => {
                    if i == 0 {
                        // Save ErrShortSrc in err. All other errors take precedence.
                        err = Some(TransformError::ShortSrc);
                    } else {
                        // Source bytes were depleted before filling up the destination buffer.
                        // Verify we made some progress, move the remaining bytes to the errStart
                        // and try to get more source bytes.
                        let (p, n) = (self.link[i].p, self.link[i].n);
                        if need_progress && n_src == 0 || n - p == in_len {
                            // There were not enough source bytes to proceed while the source
                            // buffer cannot hold any more bytes. Return a fatal error as this
                            // transformation can never complete.
                            self.fatal_error(i, TransformError::ShortInternal);
                        } else {
                            // in.b is an internal buffer and we can make progress.
                            let l = &mut self.link[i];
                            l.b.copy_within(p..n, 0);
                            l.p = 0;
                            l.n = n - p;
                            // fallthrough
                            if i > low {
                                i -= 1;
                                continue;
                            }
                        }
                    }
                }
                None => {
                    // if i == low, we have depleted the bytes at index i or any lower levels.
                    // In that case we increase low and i. In all other cases we decrease i to
                    // fetch more bytes before proceeding to the next index.
                    if i > low {
                        i -= 1;
                        continue;
                    }
                }
                Some(e) => self.fatal_error(i, e),
            }
            // Exhausted level low or fatal error: increase low and continue
            // to process the bytes accepted so far.
            i += 1;
            low = i;
        }

        // If c.errStart > 0, this means we found a fatal error.  We will keep
        // applying all transforms that are unaffected by this error. Once we
        // reach errStart, we will return the error.
        if self.err_start > 0 {
            for j in 1..self.err_start {
                self.link[j].p = 0;
                self.link[j].n = 0;
            }
            err = self.err.take();
            self.err_start = 0;
        }
        (self.link[last].n, self.link[0].p, err)
    }
}

/// Go: `transform.RemoveFunc(f)` — a Transformer that removes the runes r for which f(r) is
/// true (deprecated in x/text in favour of `runes.Remove`).
// Go: transform.go:RemoveFunc
pub fn remove_func<F: FnMut(i32) -> bool>(f: F) -> RemoveF<F> {
    RemoveF(f)
}

/// Go: `transform.removeF`.
pub struct RemoveF<F>(F);

impl<F: FnMut(i32) -> bool> Transformer for RemoveF<F> {
    // Go: transform.go:removeF.Reset
    fn reset(&mut self) {}

    // Go: transform.go:removeF.Transform
    fn transform(
        &mut self,
        dst: &mut [u8],
        mut src: &[u8],
        at_eof: bool,
    ) -> (usize, usize, Option<TransformError>) {
        let (mut n_dst, mut n_src, mut err) = (0, 0, None);
        while !src.is_empty() {
            let (r, sz);
            if (src[0] as i32) < utf8::RUNE_SELF {
                r = src[0] as i32;
                sz = 1;
            } else {
                (r, sz) = utf8::decode_rune(src);
                if sz == 1 {
                    // Invalid rune.
                    if !at_eof && !utf8::full_rune(src) {
                        err = Some(TransformError::ShortSrc);
                        break;
                    }
                    // We replace illegal bytes with RuneError. Not doing so might
                    // otherwise turn a sequence of invalid UTF-8 into valid UTF-8.
                    // The resulting byte sequence may subsequently contain runes
                    // for which t(r) is true that were passed unnoticed.
                    if !(self.0)(r) {
                        if n_dst + 3 > dst.len() {
                            err = Some(TransformError::ShortDst);
                            break;
                        }
                        dst[n_dst..n_dst + 3].copy_from_slice("\u{FFFD}".as_bytes());
                        n_dst += 3;
                    }
                    n_src += 1;
                    src = &src[sz..];
                    continue;
                }
            }

            if !(self.0)(r) {
                if n_dst + sz > dst.len() {
                    err = Some(TransformError::ShortDst);
                    break;
                }
                dst[n_dst..n_dst + sz].copy_from_slice(&src[..sz]);
                n_dst += sz;
            }
            n_src += sz;
            src = &src[sz..];
        }
        (n_dst, n_src, err)
    }
}

// Go: transform.go:grow
fn grow(b: &[u8], n: usize) -> Vec<u8> {
    let mut m = b.len();
    if m <= 32 {
        m = 64;
    } else if m <= 256 {
        m *= 2;
    } else {
        m += m >> 1;
    }
    let mut buf = vec![0; m];
    buf[..n].copy_from_slice(&b[..n]);
    buf
}

/// Go: `transform.initialBufSize`.
const INITIAL_BUF_SIZE: usize = 128;

/// Go: `transform.String(t, s)` over Go string bytes — the result of transforming s with t, the
/// number of bytes of s consumed and the error.
// Go: transform.go:String
pub fn string(t: &mut dyn Transformer, s: &[u8]) -> (Vec<u8>, usize, Option<TransformError>) {
    t.reset();
    if s.is_empty() {
        // Fast path for the common case for empty input. Results in about a
        // 86% reduction of running time for BenchmarkStringLowerEmpty.
        let (_, _, err) = t.transform(&mut [], &[], true);
        if err.is_none() {
            return (Vec::new(), 0, None);
        }
    }

    // Allocate only once. Note that both dst and src escape when passed to
    // Transform.
    let mut dst: Vec<u8> = vec![0; INITIAL_BUF_SIZE];
    let mut src: Vec<u8> = vec![0; INITIAL_BUF_SIZE];

    // The input string s is transformed in multiple chunks (starting with a
    // chunk size of initialBufSize). nDst and nSrc are per-chunk (or
    // per-Transform-call) indexes, pDst and pSrc are overall indexes.
    let (mut n_dst, mut n_src);
    let (mut p_dst, mut p_src) = (0usize, 0usize);
    let mut err;

    // Avoid allocation if the transformed string is identical to the original.
    // After this loop, pDst will point to the furthest point in s for which it
    // could be detected that t gives equal results, src[:nSrc] will
    // indicated the last processed chunk of s for which the output is not equal
    // and dst[:nDst] will be the transform of this chunk.
    let mut p_prefix = 0;
    loop {
        let n = src.len().min(s.len() - p_src);
        src[..n].copy_from_slice(&s[p_src..p_src + n]);
        (n_dst, n_src, err) = t.transform(&mut dst, &src[..n], p_src + n == s.len());
        p_dst += n_dst;
        p_src += n_src;

        // TODO:  let transformers implement an optional Spanner interface, akin
        // to norm's QuickSpan. This would even allow us to avoid any allocation.
        if dst[..n_dst] != src[..n_src] {
            break;
        }
        p_prefix = p_src;
        if err == Some(TransformError::ShortDst) {
            // A buffer can only be short if a transformer modifies its input.
            break;
        } else if err == Some(TransformError::ShortSrc) {
            if n_src == 0 {
                // No progress was made.
                break;
            }
            // Equal so far and !atEOF, so continue checking.
        } else if err.is_some() || p_prefix == s.len() {
            return (s[..p_prefix].to_vec(), p_prefix, err);
        }
    }
    // Post-condition: pDst == pPrefix + nDst && pSrc == pPrefix + nSrc.

    // We have transformed the first pSrc bytes of the input s to become pDst
    // transformed bytes. Those transformed bytes are discontiguous: the first
    // pPrefix of them equal s[:pPrefix] and the last nDst of them equal
    // dst[:nDst]. We copy them around, into a new dst buffer if necessary, so
    // that they become one contiguous slice: dst[:pDst].
    if p_prefix != 0 {
        if p_dst > dst.len() {
            let mut new_dst = vec![0; s.len() + n_dst - n_src];
            let k = (p_dst - p_prefix).min(n_dst);
            new_dst[p_prefix..p_prefix + k].copy_from_slice(&dst[..k]);
            dst = new_dst;
        } else {
            // Go copies within the same buffer (memmove).
            let k = (p_dst - p_prefix).min(n_dst);
            dst.copy_within(..k, p_prefix);
        }
        dst[..p_prefix].copy_from_slice(&s[..p_prefix]);
    }

    // Prevent duplicate Transform calls with atEOF being true at the end of
    // the input. Also return if we have an unrecoverable error.
    if (err.is_none() && p_src == s.len())
        || (err.is_some()
            && err != Some(TransformError::ShortDst)
            && err != Some(TransformError::ShortSrc))
    {
        dst.truncate(p_dst);
        return (dst, p_src, err);
    }

    // Transform the remaining input, growing dst and src buffers as necessary.
    loop {
        let n = src.len().min(s.len() - p_src);
        src[..n].copy_from_slice(&s[p_src..p_src + n]);
        let at_eof = p_src + n == s.len();
        let (n_dst, n_src, err) = t.transform(&mut dst[p_dst..], &src[..n], at_eof);
        p_dst += n_dst;
        p_src += n_src;

        // If we got ErrShortDst or ErrShortSrc, do not grow as long as we can
        // make progress. This may avoid excessive allocations.
        if err == Some(TransformError::ShortDst) {
            if n_dst == 0 {
                dst = grow(&dst, p_dst);
            }
        } else if err == Some(TransformError::ShortSrc) {
            if at_eof {
                dst.truncate(p_dst);
                return (dst, p_src, err);
            }
            if n_src == 0 {
                src = grow(&src, 0);
            }
        } else if err.is_some() || p_src == s.len() {
            dst.truncate(p_dst);
            return (dst, p_src, err);
        }
    }
}

/// Go: `transform.Bytes(t, b)` — the result of transforming b with t, the number of bytes of b
/// consumed and the error.
// Go: transform.go:Bytes
pub fn bytes(t: &mut dyn Transformer, b: &[u8]) -> (Vec<u8>, usize, Option<TransformError>) {
    do_append(t, 0, vec![0; b.len()], b)
}

// Go: transform.go:doAppend
fn do_append(
    t: &mut dyn Transformer,
    mut p_dst: usize,
    mut dst: Vec<u8>,
    src: &[u8],
) -> (Vec<u8>, usize, Option<TransformError>) {
    t.reset();
    let mut p_src = 0;
    loop {
        let (n_dst, n_src, err) = t.transform(&mut dst[p_dst..], &src[p_src..], true);
        p_dst += n_dst;
        p_src += n_src;
        if err != Some(TransformError::ShortDst) {
            dst.truncate(p_dst);
            return (dst, p_src, err);
        }

        // Grow the destination buffer, but do not grow as long as we can make
        // progress. This may avoid excessive allocations.
        if n_dst == 0 {
            dst = grow(&dst, p_dst);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Upper;

    impl Transformer for Upper {
        fn transform(
            &mut self,
            dst: &mut [u8],
            src: &[u8],
            _at_eof: bool,
        ) -> (usize, usize, Option<TransformError>) {
            let n = dst.len().min(src.len());
            for i in 0..n {
                dst[i] = src[i].to_ascii_uppercase();
            }
            (n, n, (n < src.len()).then_some(TransformError::ShortDst))
        }
        fn reset(&mut self) {}
    }

    #[test]
    fn chain_string_bytes() {
        let mut c = chain(vec![
            Box::new(Upper),
            Box::new(remove_func(|r| r == 'L' as i32)),
        ]);
        assert_eq!(string(&mut c, b"hello").0, b"HEO");
        let long = "hello ".repeat(2000);
        let want = "HEO ".repeat(2000);
        assert_eq!(string(&mut c, long.as_bytes()).0, want.as_bytes());
        assert_eq!(bytes(&mut c, long.as_bytes()).0, want.as_bytes());
        assert_eq!(string(&mut c, b"").0, b"");
        let mut rf = remove_func(|r| r == 'x' as i32);
        assert_eq!(string(&mut rf, b"axb\xff").0, "ab\u{fffd}".as_bytes());
    }
}
