window.BENCHMARK_DATA = {
  "lastUpdate": 1604225707845,
  "repoUrl": "https://github.com/neohugo/neohugo",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "blackb1rd.mov@gmail.com",
            "name": "blackb1rd",
            "username": "blackb1rd"
          },
          "committer": {
            "email": "blackb1rd.mov@gmail.com",
            "name": "blackb1rd",
            "username": "blackb1rd"
          },
          "distinct": true,
          "id": "9d4b32402e734323ddf244f24df9fc2783e752c6",
          "message": "ci: fix benchmark and specific versoin to avoid unstable result",
          "timestamp": "2020-11-01T17:03:24+07:00",
          "tree_id": "00f96342d21683a210dc18e79ef296c4c9e27138",
          "url": "https://github.com/neohugo/neohugo/commit/9d4b32402e734323ddf244f24df9fc2783e752c6"
        },
        "date": 1604225705588,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 27.1,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "43288320 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 31.4,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "46489494 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4822,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "279951 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 88,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "14708030 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10223,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "104532 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 10155,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "118453 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 54162,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "21882 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 52687,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "21360 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 4906,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "568146 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7297,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "168526 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 440350,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2486 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 25.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "47318644 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 357,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3450100 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 103652118,
            "unit": "ns/op\t36043616 B/op\t  801808 allocs/op",
            "extra": "13 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 233808,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "457773 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 704,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1669759 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1479,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "839517 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 892,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1302963 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 797,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1517899 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3372,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "368976 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 136,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8933684 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12378,
            "unit": "ns/op\t    6337 B/op\t     112 allocs/op",
            "extra": "93872 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1708,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "702854 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 296,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4249867 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2161,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "473007 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 137549,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "8818 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17030,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "70819 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 19682,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "60340 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 133677,
            "unit": "ns/op\t   20189 B/op\t     195 allocs/op",
            "extra": "9444 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6694,
            "unit": "ns/op\t    1994 B/op\t      46 allocs/op",
            "extra": "172372 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2562,
            "unit": "ns/op\t     503 B/op\t       2 allocs/op",
            "extra": "463996 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 214,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5669790 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 22339,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "53079 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 151631,
            "unit": "ns/op\t  161622 B/op\t    1204 allocs/op",
            "extra": "7876 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 245,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "5004111 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4332,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "281500 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1812,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "686575 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 67851,
            "unit": "ns/op\t   37727 B/op\t     179 allocs/op",
            "extra": "17008 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1191,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 286,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4094986 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 598,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1953926 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.24,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "140774724 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 192,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6114348 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 213,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5673328 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3052,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "382497 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1403,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "921580 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 382,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3120861 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1065,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 120,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "9897660 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 643,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1943678 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2249,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "542583 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 722,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1648638 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2577,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "475759 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 993,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1227591 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 330,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3838694 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1204,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 357,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3143955 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1208,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 188689,
            "unit": "ns/op\t   50062 B/op\t     191 allocs/op",
            "extra": "6079 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2629,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "478614 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 228,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5164777 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1862,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "657349 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 274,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4524685 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 858,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1389986 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 462,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2716932 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 28275468,
            "unit": "ns/op\t 5459521 B/op\t   80032 allocs/op",
            "extra": "38 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 164,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7354504 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4472,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "264814 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4913,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "267261 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 116,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "10325205 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1890,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "556140 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 258,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4679475 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 7572,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "162537 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 5959,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "204062 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 5926,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "209102 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3176,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "372962 times\n2 procs"
          }
        ]
      }
    ]
  }
}