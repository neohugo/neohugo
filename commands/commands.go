// Copyright 2024 The Hugo Authors. All rights reserved.
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
	"github.com/bep/simplecobra"
)

// newExec wires up all of Hugo's CLI.
func newExec() (*simplecobra.Exec, error) {
	rootCmd := &rootCommand{
		commands: []simplecobra.Commander{
			newVersionCmd(),
			newEnvCommand(),
			newServerCommand(),
			newDeployCommand(),
			newConfigCommand(),
			newNewCommand(),
			newConvertCommand(),
			newImportCommand(),
			newListCommand(),
			newModCommands(),
			newGenCommand(),
			newReleaseCommand(),
		},
	}

	return simplecobra.New(rootCmd)
}
