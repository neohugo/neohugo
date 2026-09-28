// Package hsupport holds what the nh-helpers oracles share: loading a site
// configuration from a temporary directory with allconfig.LoadConfig, string
// encoding, panic capture and the gzip-compressed JSON fixture writer.
package hsupport

import (
	"encoding/hex"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"unicode/utf8"

	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/allconfig"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/spf13/afero"
)

// Str returns s as a JSON-able value: the string itself when it is valid
// UTF-8, else {"hex": ...}.
func Str(s string) any {
	if utf8.ValidString(s) {
		return s
	}
	return map[string]string{"hex": hex.EncodeToString([]byte(s))}
}

// Call runs f and encodes its string result, turning a panic into
// {"panic": message}.
func Call(f func() string) (out any) {
	defer func() {
		if r := recover(); r != nil {
			out = map[string]any{"panic": fmt.Sprint(r)}
		}
	}()
	return Str(f())
}

// Err encodes an error as {"err": message}, or nil.
func Err(err error) any {
	if err == nil {
		return nil
	}
	return map[string]any{"err": err.Error()}
}

// Site is a configuration loaded from a temporary site directory.
type Site struct {
	Dir   string
	Confs *allconfig.Configs
}

// LoadSite writes hugo.toml (and the given extra files) below
// <tmp>/<name> and loads it with allconfig.LoadConfig.
func LoadSite(tmp, name, toml string, files map[string]string, environ []string) (*Site, error) {
	dir := filepath.Join(tmp, name)
	if err := os.MkdirAll(dir, 0o777); err != nil {
		return nil, err
	}
	if err := os.WriteFile(filepath.Join(dir, "hugo.toml"), []byte(toml), 0o666); err != nil {
		return nil, err
	}
	keys := make([]string, 0, len(files))
	for k := range files {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		p := filepath.Join(dir, k)
		if err := os.MkdirAll(filepath.Dir(p), 0o777); err != nil {
			return nil, err
		}
		if err := os.WriteFile(p, []byte(files[k]), 0o666); err != nil {
			return nil, err
		}
	}
	flags := config.New()
	flags.Set("workingDir", dir)
	if len(environ) == 0 {
		environ = []string{"NEOHUGO_ORACLE=1"}
	}
	confs, err := allconfig.LoadConfig(allconfig.ConfigSourceDescriptor{
		Fs:       afero.NewOsFs(),
		Filename: filepath.Join(dir, "hugo.toml"),
		Flags:    flags,
		Environ:  environ,
		Logger:   loggers.NewDefault(),
	})
	if err != nil {
		return nil, err
	}
	return &Site{Dir: dir, Confs: confs}, nil
}

// WriteGz writes the cases as gzip-compressed JSON (one case per line).
func WriteGz(path string, header map[string]any, cases []map[string]any) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o777); err != nil {
		return err
	}
	return goval.WriteCasesGz(path, header, cases)
}
