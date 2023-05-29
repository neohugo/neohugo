// Copyright 2023 The Hugo Authors. All rights reserved.
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

package page_test

import (
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/testconfig"
	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/spf13/afero"
)

func newTestPathSpec() *helpers.PathSpec {
	return newTestPathSpecFor(config.New())
}

func newTestPathSpecFor(cfg config.Provider) *helpers.PathSpec {
	mfs := afero.NewMemMapFs()
	conf := testconfig.GetTestConfig(mfs, cfg)
	fs := hugofs.NewFrom(mfs, conf.BaseConfig())
	ps, err := helpers.NewPathSpec(fs, conf, loggers.NewErrorLogger())
	if err != nil {
		panic(err)
	}
	return ps
}
