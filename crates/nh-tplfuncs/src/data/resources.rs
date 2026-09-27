//! Port of `tpl/data/resources.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

// Skeleton: no hand-written declarations yet; port the items in the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/data/resources.go (128 lines; 0/4 funcs executed)
//    L36-98: (ns *Namespace) getRemote(cache *filecache.Cache, unmarshal func([]byte) (bool, error), req *http.Request) error
//    L101-104: getLocal(workingDir, url string, fs afero.Fs) ([]byte, error)
//    L108-124: (ns *Namespace) getResource(cache *filecache.Cache, unmarshal func(b []byte) (bool, error), req *http.Request) error
//    L126-128: isHTTPError(res *http.Response) bool
// ---------------------------------------------------------------------------
