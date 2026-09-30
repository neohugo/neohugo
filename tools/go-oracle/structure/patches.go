package main

// The hooks patched into copies of hugolib files (see overlay_hugolib.go.txt).
var patches = []patch{
	{
		// Every (page, output format) pageRenderer resolves a template for,
		// with the template (or none).
		File: "hugolib/site_render.go",
		Old: "\t\t\ts.SendError(p.errorf(err, \"failed to resolve template\"))\n" +
			"\t\t\tcontinue\n" +
			"\t\t}\n",
		New: "\t\t\ts.SendError(p.errorf(err, \"failed to resolve template\"))\n" +
			"\t\t\tcontinue\n" +
			"\t\t}\n" +
			"\t\tnhStructRenderStart(s, p, templ, found)\n",
	},
	{
		// The number of pagers of a page that paginated.
		File: "hugolib/site_render.go",
		Old:  "\t\tif p.paginator != nil && p.paginator.current != nil {\n",
		New:  "\t\tif p.paginator != nil && p.paginator.current != nil {\n\t\t\tnhStructPagers(s, p)\n",
	},
	{
		// The page/1 alias of a paginated page.
		File: "hugolib/site_render.go",
		Old:  "\t\tif err := s.writeDestAlias(targetPaths.TargetFilename, p.Permalink(), f, p); err != nil {\n",
		New:  "\t\tnhStructPagerAlias(s, targetPaths.TargetFilename)\n\t\tif err := s.writeDestAlias(targetPaths.TargetFilename, p.Permalink(), f, p); err != nil {\n",
	},
	{
		// The template wrote something: the file is published.
		File: "hugolib/site.go",
		Old:  "\tisHTML := of.IsHTML\n",
		New:  "\tnhStructWritten(s, p, statCounter)\n\tisHTML := of.IsHTML\n",
	},
	{
		// Every alias file.
		File: "hugolib/alias.go",
		Old:  "\ttargetPath, err := handler.targetPathAlias(path)\n\tif err != nil {\n\t\treturn err\n\t}\n",
		New:  "\ttargetPath, err := handler.targetPathAlias(path)\n\tif err != nil {\n\t\treturn err\n\t}\n\tnhStructAlias(s, path, targetPath, permalink, outputFormat, p)\n",
	},
	{
		// The sites of the build.
		File: "hugolib/hugo_sites_build.go",
		Old:  "func (h *HugoSites) Build(config BuildCfg, events ...fsnotify.Event) error {\n",
		New:  "func (h *HugoSites) Build(config BuildCfg, events ...fsnotify.Event) error {\n\tnhStructSetSites(h)\n",
	},
}
