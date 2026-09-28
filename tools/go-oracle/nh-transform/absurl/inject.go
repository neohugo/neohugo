package main

import (
	"bytes"
	"io"
	"log"
	"net/url"
	"path/filepath"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-publisher/osupport"
	"github.com/neohugo/neohugo/transform"
	"github.com/neohugo/neohugo/transform/livereloadinject"
	"github.com/neohugo/neohugo/transform/metainject"
)

// The livereload and generator-tag transformers (server only / never active
// in neohugo builds, but ported): inject.jsonl.gz.

type fromTo struct {
	from *bytes.Buffer
	to   *bytes.Buffer
}

func (ft fromTo) From() transform.BytesReader { return ft.from }
func (ft fromTo) To() io.Writer               { return ft.to }

var injectDocs = []string{
	"", "<html>", "<head>", "<HEAD>", "<!DOCTYPE html><html><head><title>t</title></head></html>",
	"<!doctype html>\n<html lang=\"en\">\n  <head>\n<meta charset=utf-8>", " \t\n<!-- c --> <?xml x?><!DOCTYPE x><!-- d --><HTML class=x><HEAD id=y>body",
	"<!DOCTYPEhtml><html><head>", "<!DOCTYPE html", "<html x", "<htmlx><head>", "<!-- unterminated <html><head>",
	"<?pi <html><head>", "<head><head>", "no tags at all", "<meta name=\"generator\" content=\"x\"><head>",
	"<head><META NAME='generator' content=y>", "<meta   name=|generator|>", "<meta\tname=generator>", "<metaname=generator><head>",
	"<head>a<head>b", "<HEAD>A<HEAD>", "<Head>", " <html><head>", "<!DOCTYPE html>\xff<head>",
}

var injectURLs = []string{
	"http://localhost:1313/", "http://localhost:1313", "https://example.org:8443/docs/", "http://[::1]:80/a/b/",
	"http://host/p%20q/", "http://host", "http://host/<x>&y=\"z\"/",
}

func writeInject(out string) {
	w, err := osupport.Create(filepath.Join(out, "inject.jsonl.gz"))
	if err != nil {
		log.Fatal(err)
	}
	for _, d := range injectDocs {
		for _, us := range injectURLs {
			u, err := url.Parse(us)
			if err != nil {
				log.Fatal(err)
			}
			var b bytes.Buffer
			if err := livereloadinject.New(u)(fromTo{from: bytes.NewBufferString(d), to: &b}); err != nil {
				log.Fatal(err)
			}
			if err := w.Write(map[string]any{"t": "livereload", "url": us, "in": osupport.S(d), "out": osupport.B(b.Bytes())}); err != nil {
				log.Fatal(err)
			}
		}
		var b bytes.Buffer
		if err := metainject.HugoGenerator(fromTo{from: bytes.NewBufferString(d), to: &b}); err != nil {
			log.Fatal(err)
		}
		if err := w.Write(map[string]any{"t": "generator", "in": osupport.S(d), "out": osupport.B(b.Bytes())}); err != nil {
			log.Fatal(err)
		}
	}
	if err := w.Close(); err != nil {
		log.Fatal(err)
	}
	log.Printf("inject: %d cases", w.N())
}
