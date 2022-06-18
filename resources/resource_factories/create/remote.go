// Copyright 2021 The Hugo Authors. All rights reserved.
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

package create

import (
	"bufio"
	"bytes"
	"fmt"
	"io"
	"io/ioutil"
	"mime"
	"net/http"
	"net/http/httputil"
	"net/url"
	"path"
	"path/filepath"
	"strings"

	"github.com/mitchellh/mapstructure"
	"github.com/neohugo/neohugo/common/hugio"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/common/types"
	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/resources"
	"github.com/neohugo/neohugo/resources/resource"
)

type HTTPError struct {
	error
	Data map[string]any

	StatusCode int
	Body       string
}

func toHTTPError(err error, res *http.Response) *HTTPError {
	if err == nil {
		panic("err is nil")
	}
	if res == nil {
		return &HTTPError{
			error: err,
			Data:  map[string]any{},
		}
	}

	var body []byte
	body, _ = ioutil.ReadAll(res.Body)

	return &HTTPError{
		error: err,
		Data: map[string]any{
			"StatusCode":       res.StatusCode,
			"Status":           res.Status,
			"Body":             string(body),
			"TransferEncoding": res.TransferEncoding,
			"ContentLength":    res.ContentLength,
			"ContentType":      res.Header.Get("Content-Type"),
		},
	}
}

// FromRemote expects one or n-parts of a URL to a resource
// If you provide multiple parts they will be joined together to the final URL.
func (c *Client) FromRemote(uri string, optionsm map[string]any) (resource.Resource, error) {
	rURL, err := url.Parse(uri)
	if err != nil {
		return nil, fmt.Errorf("failed to parse URL for resource %s: %w", uri, err)
	}

	resourceID := calculateResourceID(uri, optionsm)

	_, httpResponse, err := c.cacheGetResource.GetOrCreate(resourceID, func() (io.ReadCloser, error) {
		options, err := decodeRemoteOptions(optionsm)
		if err != nil {
			return nil, fmt.Errorf("failed to decode options for resource %s: %w", uri, err)
		}
		if err := c.validateFromRemoteArgs(uri, options); err != nil {
			return nil, err
		}

		req, err := http.NewRequest(options.Method, uri, options.BodyReader())
		if err != nil {
			return nil, fmt.Errorf("failed to create request for resource %s: %w", uri, err)
		}
		addDefaultHeaders(req)

		if options.Headers != nil {
			addUserProvidedHeaders(options.Headers, req)
		}

		res, err := c.httpClient.Do(req)
		if err != nil {
			return nil, err
		}

		httpResponse, err := httputil.DumpResponse(res, true)
		if err != nil {
			return nil, toHTTPError(err, res)
		}

		if res.StatusCode != http.StatusNotFound {
			if res.StatusCode < 200 || res.StatusCode > 299 {
				return nil, toHTTPError(fmt.Errorf("failed to fetch remote resource: %s", http.StatusText(res.StatusCode)), res)
			}
		}

		return hugio.ToReadCloser(bytes.NewReader(httpResponse)), nil
	})
	if err != nil {
		return nil, err
	}
	defer httpResponse.Close()

	res, err := http.ReadResponse(bufio.NewReader(httpResponse), nil)
	if err != nil {
		return nil, err
	}

	if res.StatusCode == http.StatusNotFound {
		// Not found. This matches how looksup for local resources work.
		return nil, nil
	}

	body, err := ioutil.ReadAll(res.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read remote resource %q: %w", uri, err)
	}

	filename := path.Base(rURL.Path)
	if _, params, _ := mime.ParseMediaType(res.Header.Get("Content-Disposition")); params != nil {
		if _, ok := params["filename"]; ok {
			filename = params["filename"]
		}
	}

	var extensionHints []string

	contentType := res.Header.Get("Content-Type")

	// mime.ExtensionsByType gives a long list of extensions for text/plain,
	// just use ".txt".
	if strings.HasPrefix(contentType, "text/plain") {
		extensionHints = []string{".txt"}
	} else {
		exts, _ := mime.ExtensionsByType(contentType)
		if exts != nil {
			extensionHints = exts
		}
	}

	// Look for a file extension. If it's .txt, look for a more specific.
	if extensionHints == nil || extensionHints[0] == ".txt" {
		if ext := path.Ext(filename); ext != "" {
			extensionHints = []string{ext}
		}
	}

	// Now resolve the media type primarily using the content.
	mediaType := media.FromContent(c.rs.MediaTypes, extensionHints, body)
	if mediaType.IsZero() {
		return nil, fmt.Errorf("failed to resolve media type for remote resource %q", uri)
	}

	resourceID = filename[:len(filename)-len(path.Ext(filename))] + "_" + resourceID + mediaType.FirstSuffix.FullSuffix

	return c.rs.New(
		resources.ResourceSourceDescriptor{
			MediaType:   mediaType,
			LazyPublish: true,
			OpenReadSeekCloser: func() (hugio.ReadSeekCloser, error) {
				return hugio.NewReadSeekerNoOpCloser(bytes.NewReader(body)), nil
			},
			RelTargetFilename: filepath.Clean(resourceID),
		})
}

func (c *Client) validateFromRemoteArgs(uri string, options fromRemoteOptions) error {
	if err := c.rs.ExecHelper.Sec().CheckAllowedHTTPURL(uri); err != nil {
		return err
	}

	if err := c.rs.ExecHelper.Sec().CheckAllowedHTTPMethod(options.Method); err != nil {
		return err
	}

	return nil
}

func calculateResourceID(uri string, optionsm map[string]any) string {
	if key, found := maps.LookupEqualFold(optionsm, "key"); found {
		return helpers.HashString(key)
	}
	return helpers.HashString(uri, optionsm)
}

func addDefaultHeaders(req *http.Request, accepts ...string) {
	for _, accept := range accepts {
		if !hasHeaderValue(req.Header, "Accept", accept) {
			req.Header.Add("Accept", accept)
		}
	}
	if !hasHeaderKey(req.Header, "User-Agent") {
		req.Header.Add("User-Agent", "Hugo Static Site Generator")
	}
}

func addUserProvidedHeaders(headers map[string]any, req *http.Request) {
	if headers == nil {
		return
	}
	for key, val := range headers {
		vals := types.ToStringSlicePreserveString(val)
		for _, s := range vals {
			req.Header.Add(key, s)
		}
	}
}

func hasHeaderValue(m http.Header, key, value string) bool {
	var s []string
	var ok bool

	if s, ok = m[key]; !ok {
		return false
	}

	for _, v := range s {
		if v == value {
			return true
		}
	}
	return false
}

func hasHeaderKey(m http.Header, key string) bool {
	_, ok := m[key]
	return ok
}

type fromRemoteOptions struct {
	Method  string
	Headers map[string]any
	Body    []byte
}

func (o fromRemoteOptions) BodyReader() io.Reader {
	if o.Body == nil {
		return nil
	}
	return bytes.NewBuffer(o.Body)
}

func decodeRemoteOptions(optionsm map[string]any) (fromRemoteOptions, error) {
	options := fromRemoteOptions{
		Method: "GET",
	}

	err := mapstructure.WeakDecode(optionsm, &options)
	if err != nil {
		return options, err
	}
	options.Method = strings.ToUpper(options.Method)

	return options, nil
}
