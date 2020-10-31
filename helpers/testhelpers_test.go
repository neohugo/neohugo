package helpers

import (
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/spf13/afero"
	"github.com/spf13/viper"

	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/langs"
	"github.com/neohugo/neohugo/modules"
)

func newTestPathSpec(fs *hugofs.Fs, v *viper.Viper) *PathSpec {
	l := langs.NewDefaultLanguage(v)
	ps, _ := NewPathSpec(fs, l, nil)
	return ps
}

func newTestDefaultPathSpec(configKeyValues ...interface{}) *PathSpec {
	v := viper.New()
	fs := hugofs.NewMem(v)
	cfg := newTestCfgFor(fs)

	for i := 0; i < len(configKeyValues); i += 2 {
		cfg.Set(configKeyValues[i].(string), configKeyValues[i+1])
	}
	return newTestPathSpec(fs, cfg)
}

func newTestCfgFor(fs *hugofs.Fs) *viper.Viper {
	v := newTestCfg()
	v.SetFs(fs.Source)

	return v

}

func newTestCfg() *viper.Viper {
	v := viper.New()
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
	v := viper.New()
	spec, err := NewContentSpec(v, loggers.NewErrorLogger(), afero.NewMemMapFs())
	if err != nil {
		panic(err)
	}
	return spec
}
