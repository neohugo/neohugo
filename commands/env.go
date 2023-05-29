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

package commands

import (
	"context"
	"runtime"

	"github.com/bep/simplecobra"
	"github.com/neohugo/neohugo/common/neohugo"
)

func newEnvCommand() simplecobra.Commander {
	return &simpleCommand{
		name:  "env",
		short: "Print Hugo version and environment info",
		long:  "Print Hugo version and environment info. This is useful in Hugo bug reports",
		run: func(ctx context.Context, cd *simplecobra.Commandeer, r *rootCommand, args []string) error {
			r.Printf("%s\n", neohugo.BuildVersionString())
			r.Printf("GOOS=%q\n", runtime.GOOS)
			r.Printf("GOARCH=%q\n", runtime.GOARCH)
			r.Printf("GOVERSION=%q\n", runtime.Version())

			if r.verbose {
				deps := neohugo.GetDependencyList()
				for _, dep := range deps {
					r.Printf("%s\n", dep)
				}
			} else {
				// These are also included in the GetDependencyList above;
				// always print these as these are most likely the most useful to know about.
				deps := neohugo.GetDependencyListNonGo()
				for _, dep := range deps {
					r.Printf("%s\n", dep)
				}
			}
			return nil
		},
	}
}

func newVersionCmd() simplecobra.Commander {
	return &simpleCommand{
		name: "version",
		run: func(ctx context.Context, cd *simplecobra.Commandeer, r *rootCommand, args []string) error {
			r.Println(neohugo.BuildVersionString())
			return nil
		},
		short: "Print Hugo version and environment info",
		long:  "Print Hugo version and environment info. This is useful in Hugo bug reports.",
	}
}
