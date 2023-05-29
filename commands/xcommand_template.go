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
	"fmt"

	"github.com/bep/simplecobra"
	"github.com/spf13/cobra"
)

// nolint
func newSimpleTemplateCommand() simplecobra.Commander {
	return &simpleCommand{
		name: "template",
		run: func(ctx context.Context, cd *simplecobra.Commandeer, r *rootCommand, args []string) error {
			return nil
		},
		withc: func(cmd *cobra.Command, r *rootCommand) {
		},
	}
}

// nolint
func newTemplateCommand() *templateCommand {
	return &templateCommand{
		commands: []simplecobra.Commander{},
	}
}

// nolint
type templateCommand struct {
	r *rootCommand

	commands []simplecobra.Commander
}

// nolint
func (c *templateCommand) Commands() []simplecobra.Commander {
	return c.commands
}

// nolint
func (c *templateCommand) Name() string {
	return "template"
}

// nolint
func (c *templateCommand) Run(ctx context.Context, cd *simplecobra.Commandeer, args []string) error {
	conf, err := c.r.ConfigFromProvider(c.r.configVersionID.Load(), flagsToCfg(cd, nil))
	if err != nil {
		return err
	}
	fmt.Println("templateCommand.Run", conf)

	return nil
}

// nolint
func (c *templateCommand) Init(cd *simplecobra.Commandeer) error {
	cmd := cd.CobraCommand
	cmd.Short = "Print the site configuration"
	cmd.Long = `Print the site configuration, both default and custom settings.`
	return nil
}

// nolint
func (c *templateCommand) PreRun(cd, runner *simplecobra.Commandeer) error {
	c.r = cd.Root.Command.(*rootCommand)
	return nil
}
