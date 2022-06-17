// Copyright 2017 The Hugo Authors. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package i18n

import (
	"encoding/json"
	"fmt"
	"strings"

	"github.com/neohugo/neohugo/common/paths"
	_errors "github.com/pkg/errors"

	"github.com/neohugo/neohugo/common/herrors"
	"golang.org/x/text/language"
	yaml "gopkg.in/yaml.v2"

	"github.com/gohugoio/go-i18n/v2/i18n"
	"github.com/neohugo/neohugo/helpers"
	toml "github.com/pelletier/go-toml/v2"

	"github.com/neohugo/neohugo/deps"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/source"
)

// TranslationProvider provides translation handling, i.e. loading
// of bundles etc.
type TranslationProvider struct {
	t Translator
}

// NewTranslationProvider creates a new translation provider.
func NewTranslationProvider() *TranslationProvider {
	return &TranslationProvider{}
}

// Update updates the i18n func in the provided Deps.
func (tp *TranslationProvider) Update(d *deps.Deps) error {
	spec := source.NewSourceSpec(d.PathSpec, nil, nil)

	bundle := i18n.NewBundle(language.English)
	bundle.RegisterUnmarshalFunc("toml", toml.Unmarshal)
	bundle.RegisterUnmarshalFunc("yaml", yaml.Unmarshal)
	bundle.RegisterUnmarshalFunc("yml", yaml.Unmarshal)
	bundle.RegisterUnmarshalFunc("json", json.Unmarshal)

	// The source dirs are ordered so the most important comes first. Since this is a
	// last key win situation, we have to reverse the iteration order.
	dirs := d.BaseFs.I18n.Dirs
	for i := len(dirs) - 1; i >= 0; i-- {
		dir := dirs[i]
		src := spec.NewFilesystemFromFileMetaInfo(dir)
		files, err := src.Files()
		if err != nil {
			return err
		}
		for _, file := range files {
			if err := addTranslationFile(bundle, file); err != nil {
				return err
			}
		}
	}

	tp.t = NewTranslator(bundle, d.Cfg, d.Log)

	d.Translate = tp.t.Func(d.Language.Lang)

	return nil
}

const artificialLangTagPrefix = "art-x-"

func addTranslationFile(bundle *i18n.Bundle, r source.File) error {
	f, err := r.FileInfo().Meta().Open()
	if err != nil {
		return fmt.Errorf("failed to open translations file %q:: %w", r.LogicalName(), err)
	}

	b, err := helpers.ReaderToBytes(f)
	if err != nil {
		return _errors.Wrapf(err, "failed to read translations file %q:", r.LogicalName())
	}
	f.Close()

	name := r.LogicalName()
	lang := paths.Filename(name)
	tag := language.Make(lang)
	if tag == language.Und {
		try := artificialLangTagPrefix + lang
		_, err = language.Parse(try)
		if err != nil {
			return fmt.Errorf("%q: %s", try, err)
		}
		name = artificialLangTagPrefix + name
	}

	_, err = bundle.ParseMessageFileBytes(b, name)
	if err != nil {
		if strings.Contains(err.Error(), "no plural rule") {
			// https://github.com/neohugo/neohugo/issues/7798
			name = artificialLangTagPrefix + name
			_, err = bundle.ParseMessageFileBytes(b, name)
			if err == nil {
				return nil
			}
		}
		return errWithFileContext(fmt.Errorf("failed to load translations: %w", err), r)
	}

	return nil
}

// Clone sets the language func for the new language.
func (tp *TranslationProvider) Clone(d *deps.Deps) error {
	d.Translate = tp.t.Func(d.Language.Lang)

	return nil
}

func errWithFileContext(inerr error, r source.File) error {
	//nolint
	fim, ok := r.FileInfo().(hugofs.FileMetaInfo)
	if !ok {
		return inerr
	}

	meta := fim.Meta()
	realFilename := meta.Filename
	f, err := meta.Open()
	if err != nil {
		return inerr
	}
	defer f.Close()

	return herrors.NewFileErrorFromName(inerr, realFilename).UpdateContent(f, nil)
}
