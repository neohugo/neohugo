// Copyright 2020 The Hugo Authors. All rights reserved.
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

package hugolib

import (
	"bytes"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"testing"

	jww "github.com/spf13/jwalterweatherman"

	"github.com/neohugo/neohugo/htesting"

	"github.com/spf13/viper"

	qt "github.com/frankban/quicktest"

	"github.com/neohugo/neohugo/hugofs"

	"github.com/neohugo/neohugo/common/loggers"
)

func TestResourceChainBabel(t *testing.T) {
	if !isCI() {
		t.Skip("skip (relative) long running modules test when running locally")
	}

	if runtime.GOOS == "windows" {
		t.Skip("skip npm test on Windows")
	}

	c := qt.New(t)
	wd, err := os.Getwd()
	c.Assert(err, qt.IsNil)
	defer func() {

		c.Assert(os.Chdir(wd), qt.IsNil)
	}()

	packageJSON := `{
  "scripts": {},

  "devDependencies": {
    "@babel/cli": "7.8.4",
    "@babel/core": "7.9.0",
    "@babel/preset-env": "7.9.5"
  }
}
`

	babelConfig := `
console.error("Hugo Environment:", process.env.HUGO_ENVIRONMENT );

module.exports = {
  presets: ["@babel/preset-env"],
};

`

	js := `
/* A Car */
class Car {
  constructor(brand) {
    this.carname = brand;
  }
}
`

	workDir, clean, err := htesting.CreateTempDir(hugofs.Os, "hugo-test-babel")
	c.Assert(err, qt.IsNil)
	defer clean()

	var logBuf bytes.Buffer
	logger := loggers.NewBasicLoggerForWriter(jww.LevelInfo, &logBuf)

	v := viper.New()
	v.Set("workingDir", workDir)
	v.Set("disableKinds", []string{"taxonomy", "term", "page"})
	b := newTestSitesBuilder(t).WithLogger(logger)

	// Need to use OS fs for this.
	b.Fs = hugofs.NewDefault(v)
	b.WithWorkingDir(workDir)
	b.WithViper(v)
	b.WithContent("p1.md", "")

	b.WithTemplates("index.html", `
{{ $options := dict "noComments" true }}
{{ $transpiled := resources.Get "js/main.js" | babel -}}
Transpiled: {{ $transpiled.Content | safeJS }}

`)

	jsDir := filepath.Join(workDir, "assets", "js")
	b.Assert(os.MkdirAll(jsDir, 0777), qt.IsNil)
	b.WithSourceFile("assets/js/main.js", js)
	b.WithSourceFile("package.json", packageJSON)
	b.WithSourceFile("babel.config.js", babelConfig)

	b.Assert(os.Chdir(workDir), qt.IsNil)
	_, err = exec.Command("npm", "install").CombinedOutput()
	b.Assert(err, qt.IsNil)

	b.Build(BuildCfg{})

	// Make sure Node sees this.
	b.Assert(logBuf.String(), qt.Contains, "babel: Hugo Environment: production")
	b.Assert(err, qt.IsNil)

	b.AssertFileContent("public/index.html", `
var Car = function Car(brand) {
 _classCallCheck(this, Car);

 this.carname = brand;
};
`)

}
