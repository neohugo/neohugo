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
	"context"
	"errors"
	"fmt"
	"io"
	"log"
	"os"
	"os/signal"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"sync/atomic"
	"syscall"
	"time"

	"go.uber.org/automaxprocs/maxprocs"

	"github.com/bep/clocks"
	"github.com/bep/lazycache"
	"github.com/bep/logg"
	"github.com/bep/overlayfs"
	"github.com/bep/simplecobra"

	"github.com/neohugo/neohugo/common/hstrings"
	"github.com/neohugo/neohugo/common/htime"
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/common/neohugo"
	"github.com/neohugo/neohugo/common/paths"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/allconfig"
	"github.com/neohugo/neohugo/deps"
	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/hugolib"
	"github.com/spf13/afero"
	"github.com/spf13/cobra"
)

var errHelp = errors.New("help requested")

// Execute executes a command.
func Execute(args []string) error {
	// Default GOMAXPROCS to be CPU limit aware, still respecting GOMAXPROCS env.
	maxprocs.Set() // nolint
	x, err := newExec()
	if err != nil {
		return err
	}
	args = mapLegacyArgs(args)
	cd, err := x.Execute(context.Background(), args)
	if err != nil {
		if err == errHelp {
			cd.CobraCommand.Help() // nolint
			fmt.Println()
			return nil
		}
		if simplecobra.IsCommandError(err) {
			// Print the help, but also return the error to fail the command.
			cd.CobraCommand.Help() // nolint
			fmt.Println()
		}
	}
	return err
}

type commonConfig struct {
	mu      *sync.Mutex
	configs *allconfig.Configs
	cfg     config.Provider
	fs      *hugofs.Fs
}

// This is the root command.
type rootCommand struct {
	Printf  func(format string, v ...interface{})
	Println func(a ...interface{})
	Out     io.Writer

	logger loggers.Logger

	// The main cache busting key for the caches below.
	configVersionID atomic.Int32

	// Some, but not all commands need access to these.
	// Some needs more than one, so keep them in a small cache.
	commonConfigs *lazycache.Cache[int32, *commonConfig]
	hugoSites     *lazycache.Cache[int32, *hugolib.HugoSites]

	commands []simplecobra.Commander

	// Flags
	source      string
	buildWatch  bool
	environment string

	// Common build flags.
	baseURL         string
	gc              bool
	poll            string
	forceSyncStatic bool

	// Profile flags (for debugging of performance problems)
	cpuprofile   string
	memprofile   string
	mutexprofile string
	traceprofile string
	printm       bool

	logLevel string

	verbose bool
	debug   bool
	quiet   bool

	renderToMemory bool

	cfgFile string
	cfgDir  string
}

func (r *rootCommand) isVerbose() bool {
	return r.logger.Level() <= logg.LevelInfo
}

func (r *rootCommand) Build(cd *simplecobra.Commandeer, bcfg hugolib.BuildCfg, cfg config.Provider) (*hugolib.HugoSites, error) {
	h, err := r.Hugo(cfg)
	if err != nil {
		return nil, err
	}
	if err := h.Build(bcfg); err != nil {
		return nil, err
	}

	return h, nil
}

func (r *rootCommand) Commands() []simplecobra.Commander {
	return r.commands
}

func (r *rootCommand) ConfigFromConfig(key int32, oldConf *commonConfig) (*commonConfig, error) {
	cc, _, err := r.commonConfigs.GetOrCreate(key, func(key int32) (*commonConfig, error) {
		fs := oldConf.fs
		configs, err := allconfig.LoadConfig(
			allconfig.ConfigSourceDescriptor{
				Flags:       oldConf.cfg,
				Fs:          fs.Source,
				Filename:    r.cfgFile,
				ConfigDir:   r.cfgDir,
				Logger:      r.logger,
				Environment: r.environment,
			},
		)
		if err != nil {
			return nil, err
		}

		if !configs.Base.C.Clock.IsZero() {
			// TODO(bep) find a better place for this.
			htime.Clock = clocks.Start(configs.Base.C.Clock)
		}

		return &commonConfig{
			mu:      oldConf.mu,
			configs: configs,
			cfg:     oldConf.cfg,
			fs:      fs,
		}, nil
	})

	return cc, err
}

func (r *rootCommand) ConfigFromProvider(key int32, cfg config.Provider) (*commonConfig, error) {
	if cfg == nil {
		panic("cfg must be set")
	}
	cc, _, err := r.commonConfigs.GetOrCreate(key, func(key int32) (*commonConfig, error) {
		var dir string
		if r.source != "" {
			dir, _ = filepath.Abs(r.source)
		} else {
			dir, _ = os.Getwd()
		}

		if cfg == nil {
			cfg = config.New()
		}

		if !cfg.IsSet("workingDir") {
			cfg.Set("workingDir", dir)
		} else {
			if err := os.MkdirAll(cfg.GetString("workingDir"), 0o777); err != nil {
				return nil, fmt.Errorf("failed to create workingDir: %w", err)
			}
		}

		// Load the config first to allow publishDir to be configured in config file.
		configs, err := allconfig.LoadConfig(
			allconfig.ConfigSourceDescriptor{
				Flags:       cfg,
				Fs:          hugofs.Os,
				Filename:    r.cfgFile,
				ConfigDir:   r.cfgDir,
				Environment: r.environment,
				Logger:      r.logger,
			},
		)
		if err != nil {
			return nil, err
		}

		base := configs.Base

		cfg.Set("publishDir", base.PublishDir)
		cfg.Set("publishDirStatic", base.PublishDir)
		cfg.Set("publishDirDynamic", base.PublishDir)

		renderStaticToDisk := cfg.GetBool("renderStaticToDisk")

		sourceFs := hugofs.Os
		var destinationFs afero.Fs
		if cfg.GetBool("renderToMemory") {
			destinationFs = afero.NewMemMapFs()
			if renderStaticToDisk {
				// Hybrid, render dynamic content to Root.
				cfg.Set("publishDirDynamic", "/")
			} else {
				// Rendering to memoryFS, publish to Root regardless of publishDir.
				cfg.Set("publishDirDynamic", "/")
				cfg.Set("publishDirStatic", "/")
			}
		} else {
			destinationFs = hugofs.Os
		}

		fs := hugofs.NewFromSourceAndDestination(sourceFs, destinationFs, cfg)

		if renderStaticToDisk {
			dynamicFs := fs.PublishDir
			publishDirStatic := cfg.GetString("publishDirStatic")
			workingDir := cfg.GetString("workingDir")
			absPublishDirStatic := paths.AbsPathify(workingDir, publishDirStatic)
			staticFs := hugofs.NewBasePathFs(afero.NewOsFs(), absPublishDirStatic)

			// Serve from both the static and dynamic fs,
			// the first will take priority.
			// THis is a read-only filesystem,
			// we do all the writes to
			// fs.Destination and fs.DestinationStatic.
			fs.PublishDirServer = overlayfs.New(
				overlayfs.Options{
					Fss: []afero.Fs{
						dynamicFs,
						staticFs,
					},
				},
			)
			fs.PublishDirStatic = staticFs

		}

		if !base.C.Clock.IsZero() {
			// TODO(bep) find a better place for this.
			htime.Clock = clocks.Start(configs.Base.C.Clock)
		}

		if base.PrintPathWarnings {
			// Note that we only care about the "dynamic creates" here,
			// so skip the static fs.
			fs.PublishDir = hugofs.NewCreateCountingFs(fs.PublishDir)
		}

		commonConfig := &commonConfig{
			mu:      &sync.Mutex{},
			configs: configs,
			cfg:     cfg,
			fs:      fs,
		}

		return commonConfig, nil
	})

	return cc, err
}

func (r *rootCommand) HugFromConfig(conf *commonConfig) (*hugolib.HugoSites, error) {
	h, _, err := r.hugoSites.GetOrCreate(r.configVersionID.Load(), func(key int32) (*hugolib.HugoSites, error) {
		depsCfg := deps.DepsCfg{Configs: conf.configs, Fs: conf.fs, LogOut: r.logger.Out(), LogLevel: r.logger.Level()}
		return hugolib.NewHugoSites(depsCfg)
	})
	return h, err
}

func (r *rootCommand) Hugo(cfg config.Provider) (*hugolib.HugoSites, error) {
	h, _, err := r.hugoSites.GetOrCreate(r.configVersionID.Load(), func(key int32) (*hugolib.HugoSites, error) {
		conf, err := r.ConfigFromProvider(key, cfg)
		if err != nil {
			return nil, err
		}
		depsCfg := deps.DepsCfg{Configs: conf.configs, Fs: conf.fs, LogOut: r.logger.Out(), LogLevel: r.logger.Level()}
		return hugolib.NewHugoSites(depsCfg)
	})
	return h, err
}

func (r *rootCommand) Name() string {
	return "hugo"
}

func (r *rootCommand) Run(ctx context.Context, cd *simplecobra.Commandeer, args []string) error {
	if !r.buildWatch {
		defer r.timeTrack(time.Now(), "Total")
	}

	b := newHugoBuilder(r, nil)

	if err := b.loadConfig(cd, false); err != nil {
		return err
	}

	err := func() error {
		if r.buildWatch {
			defer r.timeTrack(time.Now(), "Built")
		}
		err := b.build()
		if err != nil {
			return err
		}
		return nil
	}()
	if err != nil {
		return err
	}

	if !r.buildWatch {
		// Done.
		return nil
	}

	watchDirs, err := b.getDirList()
	if err != nil {
		return err
	}

	watchGroups := helpers.ExtractAndGroupRootPaths(watchDirs)

	for _, group := range watchGroups {
		r.Printf("Watching for changes in %s\n", group)
	}
	watcher, err := b.newWatcher(r.poll, watchDirs...)
	if err != nil {
		return err
	}

	defer watcher.Close()

	r.Println("Press Ctrl+C to stop")

	sigs := make(chan os.Signal, 1)
	signal.Notify(sigs, syscall.SIGINT, syscall.SIGTERM)

	<-sigs

	return nil
}

func (r *rootCommand) PreRun(cd, runner *simplecobra.Commandeer) error {
	r.Out = os.Stdout
	if r.quiet {
		r.Out = io.Discard
	}
	// Used by mkcert (server).
	log.SetOutput(r.Out)

	r.Printf = func(format string, v ...interface{}) {
		if !r.quiet {
			fmt.Fprintf(r.Out, format, v...)
		}
	}
	r.Println = func(a ...interface{}) {
		if !r.quiet {
			fmt.Fprintln(r.Out, a...)
		}
	}
	_, running := runner.Command.(*serverCommand)
	var err error
	r.logger, err = r.createLogger(running)
	if err != nil {
		return err
	}

	r.commonConfigs = lazycache.New(lazycache.Options[int32, *commonConfig]{MaxEntries: 5})
	// We don't want to keep stale HugoSites in memory longer than needed.
	r.hugoSites = lazycache.New(lazycache.Options[int32, *hugolib.HugoSites]{
		MaxEntries: 1,
		OnEvict: func(key int32, value *hugolib.HugoSites) {
			value.Close()
			runtime.GC()
		},
	})

	return nil
}

func (r *rootCommand) createLogger(running bool) (loggers.Logger, error) {
	level := logg.LevelWarn

	if r.logLevel != "" {
		switch strings.ToLower(r.logLevel) {
		case "debug":
			level = logg.LevelDebug
		case "info":
			level = logg.LevelInfo
		case "warn", "warning":
			level = logg.LevelWarn
		case "error":
			level = logg.LevelError
		default:
			return nil, fmt.Errorf("invalid log level: %q, must be one of debug, warn, info or error", r.logLevel)
		}
	} else {
		if r.verbose {
			neohugo.Deprecate("--verbose", "use --logLevel info", "v0.114.0")
			neohugo.Deprecate("--verbose", "use --logLevel info", "v0.114.0")
			level = logg.LevelInfo
		}

		if r.debug {
			neohugo.Deprecate("--debug", "use --logLevel debug", "v0.114.0")
			level = logg.LevelDebug
		}
	}

	optsLogger := loggers.Options{
		DistinctLevel: logg.LevelWarn,
		Level:         level,
		Stdout:        r.Out,
		Stderr:        r.Out,
		StoreErrors:   running,
	}

	return loggers.New(optsLogger), nil
}

func (r *rootCommand) Reset() {
	r.logger.Reset()
	loggers.Log().Reset()
}

// IsTestRun reports whether the command is running as a test.
func (r *rootCommand) IsTestRun() bool {
	return os.Getenv("HUGO_TESTRUN") != ""
}

func (r *rootCommand) Init(cd *simplecobra.Commandeer) error {
	cmd := cd.CobraCommand
	cmd.Use = "hugo [flags]"
	cmd.Short = "hugo builds your site"
	cmd.Long = `hugo is the main command, used to build your Hugo site.

Hugo is a Fast and Flexible Static Site Generator
built with love by spf13 and friends in Go.

Complete documentation is available at https://gohugo.io/.`

	// Configure persistent flags
	cmd.PersistentFlags().StringVarP(&r.source, "source", "s", "", "filesystem path to read files relative from")
	cmd.PersistentFlags().SetAnnotation("source", cobra.BashCompSubdirsInDir, []string{}) // nolint
	cmd.PersistentFlags().StringP("destination", "d", "", "filesystem path to write files to")
	cmd.PersistentFlags().SetAnnotation("destination", cobra.BashCompSubdirsInDir, []string{}) // nolint

	cmd.PersistentFlags().StringVarP(&r.environment, "environment", "e", "", "build environment")
	cmd.PersistentFlags().StringP("themesDir", "", "", "filesystem path to themes directory")
	cmd.PersistentFlags().StringP("ignoreVendorPaths", "", "", "ignores any _vendor for module paths matching the given Glob pattern")
	cmd.PersistentFlags().String("clock", "", "set the clock used by Hugo, e.g. --clock 2021-11-06T22:30:00.00+09:00")

	cmd.PersistentFlags().StringVar(&r.cfgFile, "config", "", "config file (default is hugo.yaml|json|toml)")
	cmd.PersistentFlags().StringVar(&r.cfgDir, "configDir", "config", "config dir")
	cmd.PersistentFlags().BoolVar(&r.quiet, "quiet", false, "build in quiet mode")
	cmd.PersistentFlags().BoolVar(&r.renderToMemory, "renderToMemory", false, "render to memory (mostly useful when running the server)")

	// Set bash-completion
	_ = cmd.PersistentFlags().SetAnnotation("config", cobra.BashCompFilenameExt, config.ValidConfigFileExtensions) // nolint

	cmd.PersistentFlags().BoolVarP(&r.verbose, "verbose", "v", false, "verbose output")
	cmd.PersistentFlags().BoolVarP(&r.debug, "debug", "", false, "debug output")
	cmd.PersistentFlags().StringVar(&r.logLevel, "logLevel", "", "log level (debug|info|warn|error)")
	cmd.Flags().BoolVarP(&r.buildWatch, "watch", "w", false, "watch filesystem for changes and recreate as needed")

	// Configure local flags
	applyLocalFlagsBuild(cmd, r)

	// Set bash-completion.
	// Each flag must first be defined before using the SetAnnotation() call.
	_ = cmd.Flags().SetAnnotation("source", cobra.BashCompSubdirsInDir, []string{}) // nolint

	return nil
}

// A sub set of the complete build flags. These flags are used by new and mod.
func applyLocalFlagsBuildConfig(cmd *cobra.Command, r *rootCommand) {
	cmd.Flags().StringSliceP("theme", "t", []string{}, "themes to use (located in /themes/THEMENAME/)")
	cmd.Flags().StringVarP(&r.baseURL, "baseURL", "b", "", "hostname (and path) to the root, e.g. https://spf13.com/")
	cmd.Flags().StringP("cacheDir", "", "", "filesystem path to cache directory")
	_ = cmd.Flags().SetAnnotation("cacheDir", cobra.BashCompSubdirsInDir, []string{})
	cmd.Flags().StringP("contentDir", "c", "", "filesystem path to content directory")
	_ = cmd.Flags().SetAnnotation("theme", cobra.BashCompSubdirsInDir, []string{"themes"})
}

// Flags needed to do a build (used by hugo and hugo server commands)
func applyLocalFlagsBuild(cmd *cobra.Command, r *rootCommand) {
	applyLocalFlagsBuildConfig(cmd, r)
	cmd.Flags().Bool("cleanDestinationDir", false, "remove files from destination not found in static directories")
	cmd.Flags().BoolP("buildDrafts", "D", false, "include content marked as draft")
	cmd.Flags().BoolP("buildFuture", "F", false, "include content with publishdate in the future")
	cmd.Flags().BoolP("buildExpired", "E", false, "include expired content")
	cmd.Flags().BoolP("ignoreCache", "", false, "ignores the cache directory")
	cmd.Flags().Bool("enableGitInfo", false, "add Git revision, date, author, and CODEOWNERS info to the pages")
	cmd.Flags().StringP("layoutDir", "l", "", "filesystem path to layout directory")
	cmd.Flags().BoolVar(&r.gc, "gc", false, "enable to run some cleanup tasks (remove unused cache files) after the build")
	cmd.Flags().StringVar(&r.poll, "poll", "", "set this to a poll interval, e.g --poll 700ms, to use a poll based approach to watch for file system changes")
	cmd.Flags().Bool("panicOnWarning", false, "panic on first WARNING log")
	cmd.Flags().Bool("templateMetrics", false, "display metrics about template executions")
	cmd.Flags().Bool("templateMetricsHints", false, "calculate some improvement hints when combined with --templateMetrics")
	cmd.Flags().BoolVar(&r.forceSyncStatic, "forceSyncStatic", false, "copy all files when static is changed.")
	cmd.Flags().BoolP("noTimes", "", false, "don't sync modification time of files")
	cmd.Flags().BoolP("noChmod", "", false, "don't sync permission mode of files")
	cmd.Flags().BoolP("noBuildLock", "", false, "don't create .hugo_build.lock file")
	cmd.Flags().BoolP("printI18nWarnings", "", false, "print missing translations")
	cmd.Flags().BoolP("printPathWarnings", "", false, "print warnings on duplicate target paths etc.")
	cmd.Flags().BoolP("printUnusedTemplates", "", false, "print warnings on unused templates.")
	cmd.Flags().StringVarP(&r.cpuprofile, "profile-cpu", "", "", "write cpu profile to `file`")
	cmd.Flags().StringVarP(&r.memprofile, "profile-mem", "", "", "write memory profile to `file`")
	cmd.Flags().BoolVarP(&r.printm, "printMemoryUsage", "", false, "print memory usage to screen at intervals")
	cmd.Flags().StringVarP(&r.mutexprofile, "profile-mutex", "", "", "write Mutex profile to `file`")
	cmd.Flags().StringVarP(&r.traceprofile, "trace", "", "", "write trace to `file` (not useful in general)")

	// Hide these for now.
	cmd.Flags().MarkHidden("profile-cpu")   // nolint
	cmd.Flags().MarkHidden("profile-mem")   // nolint
	cmd.Flags().MarkHidden("profile-mutex") // nolint

	cmd.Flags().StringSlice("disableKinds", []string{}, "disable different kind of pages (home, RSS etc.)")
	cmd.Flags().Bool("minify", false, "minify any supported output format (HTML, XML etc.)")
	// nolint
	_ = cmd.Flags().SetAnnotation("destination", cobra.BashCompSubdirsInDir, []string{})
}

func (r *rootCommand) timeTrack(start time.Time, name string) {
	elapsed := time.Since(start)
	r.Printf("%s in %v ms\n", name, int(1000*elapsed.Seconds()))
}

type simpleCommand struct {
	use   string
	name  string
	short string
	long  string
	run   func(ctx context.Context, cd *simplecobra.Commandeer, rootCmd *rootCommand, args []string) error
	withc func(cmd *cobra.Command, r *rootCommand)
	initc func(cd *simplecobra.Commandeer) error

	commands []simplecobra.Commander

	rootCmd *rootCommand
}

func (c *simpleCommand) Commands() []simplecobra.Commander {
	return c.commands
}

func (c *simpleCommand) Name() string {
	return c.name
}

func (c *simpleCommand) Run(ctx context.Context, cd *simplecobra.Commandeer, args []string) error {
	if c.run == nil {
		return nil
	}
	return c.run(ctx, cd, c.rootCmd, args)
}

func (c *simpleCommand) Init(cd *simplecobra.Commandeer) error {
	c.rootCmd = cd.Root.Command.(*rootCommand)
	cmd := cd.CobraCommand
	cmd.Short = c.short
	cmd.Long = c.long
	if c.use != "" {
		cmd.Use = c.use
	}
	if c.withc != nil {
		c.withc(cmd, c.rootCmd)
	}
	return nil
}

func (c *simpleCommand) PreRun(cd, runner *simplecobra.Commandeer) error {
	if c.initc != nil {
		return c.initc(cd)
	}
	return nil
}

func mapLegacyArgs(args []string) []string {
	if len(args) > 1 && args[0] == "new" && !hstrings.EqualAny(args[1], "site", "theme", "content") {
		// Insert "content" as the second argument
		args = append(args[:1], append([]string{"content"}, args[1:]...)...)
	}
	return args
}
