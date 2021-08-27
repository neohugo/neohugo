package helpers

import (
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/config"
	"github.com/spf13/afero"

	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/langs"
	"github.com/neohugo/neohugo/modules"
)

func newTestPathSpec(fs *hugofs.Fs, v config.Provider) *PathSpec {
	l := langs.NewDefaultLanguage(v)
	ps, _ := NewPathSpec(fs, l, nil)
	return ps
}

func newTestDefaultPathSpec(configKeyValues ...interface{}) *PathSpec {
	v := config.New()
	fs := hugofs.NewMem(v)
	cfg := newTestCfg()

	for i := 0; i < len(configKeyValues); i += 2 {
		cfg.Set(configKeyValues[i].(string), configKeyValues[i+1])
	}
	return newTestPathSpec(fs, cfg)
}

func newTestCfg() config.Provider {
	v := config.New()
	v.Set("contentDir", "content")
	v.Set("dataDir", "data")
	v.Set("i18nDir", "i18n")
	v.Set("layoutDir", "layouts")
	v.Set("assetDir", "assets")
	v.Set("resourceDir", "resources")
	v.Set("publishDir", "public")
	v.Set("archetypeDir", "archetypes")
	if _, err := langs.LoadLanguageSettings(v, nil); err != nil {
		panic(err)
	}
	if _, err := langs.LoadLanguageSettings(v, nil); err != nil {
		panic(err)
	}
	mod, err := modules.CreateProjectModule(v)
	if err != nil {
		panic(err)
	}
	v.Set("allModules", modules.Modules{mod})

	return v
}

func newTestContentSpec() *ContentSpec {
	v := config.New()
	spec, err := NewContentSpec(v, loggers.NewErrorLogger(), afero.NewMemMapFs())
	if err != nil {
		panic(err)
	}
	return spec
}
