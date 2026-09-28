package osupport

// MinifiersPatch lets minifiers.New run without a config provider: with a nil
// cfg it reads the `minify` section from minifiers.OracleConf. Everything else
// in New (and the registration order) is the real code.
var MinifiersPatch = Patch{
	File:   "minifiers/minifiers.go",
	After:  "func New(mediaTypes media.Types, outputFormats output.Formats, cfg config.AllProvider) (Client, error) {",
	Insert: "\n\tif cfg == nil {\n\t\tcfg = oracleCfg{}\n\t}",
	Append: `
// OracleConf is the minify config section minifiers.New uses when called with
// a nil cfg (T07 oracles).
var OracleConf MinifyConfig

type oracleCfg struct{ config.AllProvider }

func (oracleCfg) GetConfigSection(string) any { return OracleConf }
`,
}

// PublisherPatches record every Publish call (the descriptor, the bytes handed
// to the publisher and the bytes written to disk) and export the unexported
// pieces the oracles call directly: the transformer chain of a descriptor and
// the HTML elements collector.
var PublisherPatches = []Patch{
	{
		File:   "publisher/publisher.go",
		After:  "import (",
		Insert: "\n\t\"bytes\"",
	},
	{
		File:  "publisher/publisher.go",
		After: "\tsrc := d.Src\n",
		Insert: `	var oracleIn []byte
	if OracleRecord != nil {
		oracleIn, _ = io.ReadAll(d.Src)
		d.Src = bytes.NewReader(oracleIn)
		src = d.Src
	}
`,
	},
	{
		File:  "publisher/publisher.go",
		After: "\t\t// This is now what we write to disk.\n\t\tsrc = b\n\t}\n",
		Insert: `	if OracleRecord != nil {
		out := oracleIn
		if bb, ok := src.(*bytes.Buffer); ok {
			out = bb.Bytes()
		}
		OracleRecord(d, oracleIn, append([]byte(nil), out...))
	}
`,
		Append: `
// OracleRecord is set by the T07 oracles: called with every published
// descriptor, its source bytes and the bytes written to disk.
var OracleRecord func(d Descriptor, in, out []byte)

// OracleChain runs the publisher's transformer chain for d (Go's
// createTransformerChain + Chain.Apply) with the given minifier client and
// returns the bytes Publish would write.
func OracleChain(min minifiers.Client, d Descriptor) ([]byte, error) {
	p := DestinationPublisher{min: min}
	transformers := p.createTransformerChain(d)
	if len(transformers) == 0 {
		return io.ReadAll(d.Src)
	}
	var b bytes.Buffer
	if err := transformers.Apply(&b, d.Src); err != nil {
		return nil, err
	}
	return b.Bytes(), nil
}
`,
	},
	{
		File: "publisher/htmlElementsCollector.go",
		Append: `
// OracleCollect feeds each doc to a new collector writer of one collector (as
// Publish does, one Write per document) and returns getHTMLElements.
func OracleCollect(conf config.BuildStats, docs [][]byte) HTMLElements {
	c := newHTMLElementsCollector(conf)
	for _, d := range docs {
		w := newHTMLElementsCollectorWriter(c)
		_, _ = w.Write(d)
	}
	return c.getHTMLElements()
}

// OracleCollectChunks feeds the chunks to ONE collector writer, one Write per
// chunk (the writer state carries over between writes).
func OracleCollectChunks(conf config.BuildStats, chunks [][]byte) (HTMLElements, string) {
	c := newHTMLElementsCollector(conf)
	w := newHTMLElementsCollectorWriter(c)
	for _, ch := range chunks {
		_, _ = w.Write(ch)
	}
	errs := ""
	if w.err != nil {
		errs = w.err.Error()
	}
	return c.getHTMLElements(), errs
}

// OracleParseElement is parseHTMLElement of one element string.
func OracleParseElement(conf config.BuildStats, s string) (tag string, classes, ids []string, err error) {
	w := newHTMLElementsCollectorWriter(newHTMLElementsCollector(conf))
	el, err := w.parseHTMLElement(s)
	return el.Tag, el.Classes, el.IDs, err
}

// OracleIsClosedByTag is isClosedByTag.
func OracleIsClosedByTag(b, tagName []byte) bool {
	return isClosedByTag(b, tagName)
}
`,
	},
}
