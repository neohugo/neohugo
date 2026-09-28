//! Port of `resources/resource_factories/bundler/bundler.go`.
//!
//! Owner: Wave B task T15 (resource-factories).

//! Go `resource_factories/bundler`: `resources.Concat(targetPath, resources)`. Cached by TARGET PATH
//! ONLY (first caller wins for the whole build, across languages). All parts must share the media
//! type; for JavaScript parts are joined with `"\n;\n"`; the composite's media type comes from the
//! target path extension; LazyPublish.

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::Arc;

use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::hugio::{OpenReadSeekCloser, ReadSeekCloser};
use nh_media::media::media_type::MediaType;
use nh_resource::resourcetypes::{Resource, Resources};
use nh_resources::resource::ResourceSourceDescriptor;
use nh_resources::resource_spec::Spec;

/// Go: `bundler.Client` — concatenation and other bundling related tasks to Resource objects.
pub struct Client {
    pub rs: Arc<Spec>,
}

/// Go: `multiReadSeekCloser` — an `io.MultiReader` over the sources; `Seek` seeks every source
/// and restarts the multi reader.
struct MultiReadSeekCloser {
    /// Go `mr` (`io.MultiReader`): the index of the current reader.
    current: usize,
    sources: Vec<Box<dyn ReadSeekCloser>>,
}

/// Go: `toReaders(sources)` — the sources as `io.Reader`s (here the multi reader starts at the
/// first source).
// Go: resources/resource_factories/bundler/bundler.go:toReaders
fn to_readers(_sources: &[Box<dyn ReadSeekCloser>]) -> usize {
    0
}

// Go: resources/resource_factories/bundler/bundler.go:newMultiReadSeekCloser
fn new_multi_read_seek_closer(sources: Vec<Box<dyn ReadSeekCloser>>) -> MultiReadSeekCloser {
    let mr = to_readers(&sources);
    MultiReadSeekCloser {
        current: mr,
        sources,
    }
}

impl Read for MultiReadSeekCloser {
    /// Go `io.MultiReader.Read`: reads from the current source; at its EOF moves on to the next.
    // Go: resources/resource_factories/bundler/bundler.go:(*multiReadSeekCloser).Read
    fn read(&mut self, p: &mut [u8]) -> io::Result<usize> {
        while self.current < self.sources.len() {
            let n = self.sources[self.current].read(p)?;
            if n > 0 || p.is_empty() {
                return Ok(n);
            }
            self.current += 1;
        }
        Ok(0)
    }
}

impl Seek for MultiReadSeekCloser {
    // Go: resources/resource_factories/bundler/bundler.go:(*multiReadSeekCloser).Seek
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let mut new_offset = 0;
        for s in &mut self.sources {
            new_offset = s.seek(pos)?;
        }

        self.current = to_readers(&self.sources);

        Ok(new_offset)
    }
}

// Go: resources/resource_factories/bundler/bundler.go:(*multiReadSeekCloser).Close
// (Go's Close closes every source and ignores their errors: dropping the reader drops the
// sources.)

/// Go `media.Builtin.JavascriptType` (`text/javascript`).
fn is_javascript(m: &MediaType) -> bool {
    m.main_type == "text" && m.sub_type == "javascript"
}

impl Client {
    // Go: resources/resource_factories/bundler/bundler.go:New
    pub fn new(rs: Arc<Spec>) -> Client {
        Client { rs }
    }

    /// Concat concatenates the list of Resource objects.
    // Go: resources/resource_factories/bundler/bundler.go:Concat
    pub fn concat(&self, target_path: &str, r: &Resources) -> Result<Arc<dyn Resource>> {
        let target_path = go_path::path::clean(target_path).to_string();
        let rs = self.rs.clone();
        let r = r.clone();
        self.rs
            .resource_cache()
            .get_or_create(&target_path.clone(), move || {
                let mut resolvedm = MediaType::default();

                // The given set of resources must be of the same Media Type. We may improve on
                // that in the future, but then we need to know more.
                for (i, rr) in r.iter().enumerate() {
                    let mt = rr.media_type();
                    if i > 0 && mt.typ != resolvedm.typ {
                        return Err(Error::new(format!(
                            "resources in Concat must be of the same Media Type, got {} and {}",
                            go_strconv::quote(&mt.typ),
                            go_strconv::quote(&resolvedm.typ)
                        )));
                    }
                    resolvedm = mt;
                }

                // (Go's identity manager for rebuilds is not ported.)

                let parts = r.clone();
                let concatr: OpenReadSeekCloser = Arc::new(move || {
                    let mut rcsources: Vec<Box<dyn ReadSeekCloser>> = Vec::new();
                    for s in &parts {
                        let Some(rc) = s.read_seek_closer() else {
                            return Err(Error::new(format!(
                                "resource {} does not implement resource.ReadSeekerCloserResource",
                                s.tpl_type_name()
                            )));
                        };
                        // (On an error Go closes the already opened sources: dropped here.)
                        rcsources.push(rc?);
                    }

                    // Arbitrary JavaScript files require a barrier between them to be safely
                    // concatenated together. Without this, the last line of one file can affect
                    // the first line of the next file and change how both files are
                    // interpreted.
                    if is_javascript(&resolvedm) {
                        let mut readers: Vec<Box<dyn ReadSeekCloser>> =
                            Vec::with_capacity(2 * rcsources.len());
                        for (i, rc) in rcsources.into_iter().enumerate() {
                            if i > 0 {
                                readers.push(nh_common::hugio::read_seeker_from_bytes(
                                    b"\n;\n".to_vec(),
                                ));
                            }
                            readers.push(rc);
                        }
                        return Ok(Box::new(new_multi_read_seek_closer(readers))
                            as Box<dyn ReadSeekCloser>);
                    }

                    Ok(Box::new(new_multi_read_seek_closer(rcsources)) as Box<dyn ReadSeekCloser>)
                });

                let composite = rs.new_resource(ResourceSourceDescriptor {
                    lazy_publish: true,
                    open_read_seek_closer: Some(concatr),
                    target_path: target_path.clone(),
                    ..Default::default()
                })?;

                Ok(composite)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src(b: &[u8]) -> Box<dyn ReadSeekCloser> {
        nh_common::hugio::read_seeker_from_bytes(b.to_vec())
    }

    #[test]
    fn multi_read_seek_closer() {
        let mut r = new_multi_read_seek_closer(vec![src(b"ab"), src(b""), src(b"cd")]);
        let mut out = Vec::new();
        r.read_to_end(&mut out).unwrap();
        assert_eq!(out, b"abcd");
        // Go: every source is seeked and the multi reader restarts.
        assert_eq!(r.seek(SeekFrom::Start(1)).unwrap(), 1);
        let mut out = Vec::new();
        r.read_to_end(&mut out).unwrap();
        assert_eq!(out, b"bd");
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_factories/bundler/bundler.go (171 lines; 6/7 funcs executed)
//   types: Client, multiReadSeekCloser
// OK L36-38: New(rs *resources.Spec) *Client
// OK L45-51: toReaders(sources []hugio.ReadSeekCloser) []io.Reader
// OK L53-56: newMultiReadSeekCloser(sources ...hugio.ReadSeekCloser) *multiReadSeekCloser
// OK L58-60: (r *multiReadSeekCloser) Read(p []byte) (n int, err error)
// OK L62-73: (r *multiReadSeekCloser) Seek(offset int64, whence int) (newOffset int64, err error)
// OK L75-80: (r *multiReadSeekCloser) Close() error
// OK L83-171: (c *Client) Concat(targetPath string, r resource.Resources) (resource.Resource, error)
// ---------------------------------------------------------------------------
