package commands

import (
	"bytes"
	"encoding/csv"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"

	qt "github.com/frankban/quicktest"
)

func captureStdout(f func() error) (string, error) {
	old := os.Stdout
	r, w, _ := os.Pipe()
	os.Stdout = w

	if err := f(); err != nil {
		return "", err
	}

	w.Close()
	os.Stdout = old

	var buf bytes.Buffer

	if _, err := io.Copy(&buf, r); err != nil {
		return "", err
	}
	return buf.String(), nil
}

func TestListAll(t *testing.T) {
	c := qt.New(t)
	dir, clean, err := createSimpleTestSite(t, testSiteConfig{})
	defer clean()

	c.Assert(err, qt.IsNil)

	hugoCmd := newCommandsBuilder().addAll().build()
	cmd := hugoCmd.getCommand()

	defer func() {
		os.RemoveAll(dir)
	}()

	cmd.SetArgs([]string{"-s=" + dir, "list", "all"})

	out, err := captureStdout(func() error {
		_, err := cmd.ExecuteC()
		return err
	})
	c.Assert(err, qt.IsNil)

	r := csv.NewReader(strings.NewReader(out))

	header, err := r.Read()

	c.Assert(err, qt.IsNil)
	c.Assert(header, qt.DeepEquals, []string{
		"path", "slug", "title",
		"date", "expiryDate", "publishDate",
		"draft", "permalink",
	})

	record, err := r.Read()

	c.Assert(err, qt.IsNil)
	c.Assert(record, qt.DeepEquals, []string{
		filepath.Join("content", "p1.md"), "", "P1",
		"0001-01-01T00:00:00Z", "0001-01-01T00:00:00Z", "0001-01-01T00:00:00Z",
		"false", "https://example.org/p1/",
	})
}
