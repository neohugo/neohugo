// Copyright 2019 The Hugo Authors. All rights reserved.
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

package htesting

import (
	"path/filepath"

	"github.com/neohugo/neohugo/cache/filecache"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/output"
	"github.com/neohugo/neohugo/resources"
	"github.com/spf13/afero"
)

func NewTestResourceSpec() (*resources.Spec, error) {
	cfg := config.New()
	cfg.Set("baseURL", "https://example.org")
	cfg.Set("publishDir", "public")

	imagingCfg := map[string]any{
		"resampleFilter": "linear",
		"quality":        68,
		"anchor":         "left",
	}

	cfg.Set("imaging", imagingCfg)

	fs := hugofs.NewFrom(hugofs.NewBaseFileDecorator(afero.NewMemMapFs()), cfg)

	s, err := helpers.NewPathSpec(fs, cfg, nil)
	if err != nil {
		return nil, err
	}

	filecaches, err := filecache.NewCaches(s)
	if err != nil {
		return nil, err
	}

	spec, err := resources.NewSpec(s, filecaches, nil, nil, nil, nil, output.DefaultFormats, media.DefaultTypes)
	return spec, err
}

func NewResourceTransformer(filename, content string) (resources.ResourceTransformer, error) {
	spec, err := NewTestResourceSpec()
	if err != nil {
		return nil, err
	}
	return NewResourceTransformerForSpec(spec, filename, content)
}

func NewResourceTransformerForSpec(spec *resources.Spec, filename, content string) (resources.ResourceTransformer, error) {
	filename = filepath.FromSlash(filename)

	fs := spec.Fs.Source
	if err := afero.WriteFile(fs, filename, []byte(content), 0o777); err != nil {
		return nil, err
	}

	r, err := spec.New(resources.ResourceSourceDescriptor{Fs: fs, SourceFilename: filename})
	if err != nil {
		return nil, err
	}

	return r.(resources.ResourceTransformer), nil
}
