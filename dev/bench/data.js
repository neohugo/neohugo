window.BENCHMARK_DATA = {
  "lastUpdate": 1604769457899,
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
      },
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
          "id": "e1a04b4d0ea4427c4ee5203120df932a00cde71b",
          "message": "ci: fix benchmark",
          "timestamp": "2020-11-02T00:00:10+07:00",
          "tree_id": "60a6dda1f22513af58e7a68868a07ea14db6a309",
          "url": "https://github.com/neohugo/neohugo/commit/e1a04b4d0ea4427c4ee5203120df932a00cde71b"
        },
        "date": 1604250615592,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 28.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "39201847 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 24.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "55183105 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 5108,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "266926 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 91.1,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "13496298 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10175,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "108590 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 10502,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "112220 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 62773,
            "unit": "ns/op\t   31425 B/op\t      71 allocs/op",
            "extra": "20557 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 54466,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "21876 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2796,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "458449 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7064,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "159472 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 444602,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2658 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 24.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "48398368 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 371,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3326517 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 111152200,
            "unit": "ns/op\t36167567 B/op\t  803153 allocs/op",
            "extra": "12 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 13496,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "391978 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 710,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1708671 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1410,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "824454 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 937,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1291657 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 771,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1527129 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3291,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "381897 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 154,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "7840682 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12057,
            "unit": "ns/op\t    6337 B/op\t     112 allocs/op",
            "extra": "94834 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1792,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "667684 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 279,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4336086 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2172,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "518181 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 141095,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "8124 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17371,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "70608 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 21531,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "57374 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 131946,
            "unit": "ns/op\t   20190 B/op\t     195 allocs/op",
            "extra": "9600 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6343,
            "unit": "ns/op\t    1988 B/op\t      46 allocs/op",
            "extra": "184017 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2624,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "463806 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 205,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5860867 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 22937,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "53912 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 155598,
            "unit": "ns/op\t  161626 B/op\t    1204 allocs/op",
            "extra": "6582 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 239,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4907425 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4194,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "272334 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1848,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "775689 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 73016,
            "unit": "ns/op\t   37739 B/op\t     179 allocs/op",
            "extra": "15818 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1123,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "990175 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 300,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4007752 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 598,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1997295 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.1,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "138950920 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 202,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6404583 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 217,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5709808 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3064,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "379612 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1383,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "851191 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 388,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3104019 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1025,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 119,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "10213327 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 587,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "2028600 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2151,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "560722 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 720,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1730394 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2516,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "418026 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 942,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1201174 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 321,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3545481 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1150,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 330,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3567793 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1230,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "937675 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 225514,
            "unit": "ns/op\t   50062 B/op\t     191 allocs/op",
            "extra": "5601 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2577,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "496328 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 228,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5310297 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1908,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "637876 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 265,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4522216 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 835,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1327839 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 456,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2713958 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 27560098,
            "unit": "ns/op\t 5459252 B/op\t   80032 allocs/op",
            "extra": "43 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 165,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7016262 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4401,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "272340 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4672,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "280389 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 104,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "11447653 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1760,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "572262 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 264,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4827823 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 7540,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "153260 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 5828,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "204374 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 5846,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "210146 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3237,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "377612 times\n2 procs"
          }
        ]
      },
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
          "id": "8fb002e5d42606778d4d6627bc97cb8d6b5cb817",
          "message": "ci: remove git diff benchmark",
          "timestamp": "2020-11-02T00:18:54+07:00",
          "tree_id": "c749faafc9f6fb65dbc10573dd347db627d4de06",
          "url": "https://github.com/neohugo/neohugo/commit/8fb002e5d42606778d4d6627bc97cb8d6b5cb817"
        },
        "date": 1604251726479,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 31.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "32521453 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 26.4,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "37876528 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 5263,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "263260 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 103,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "13224226 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 11064,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "117249 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 11572,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "111856 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 64162,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "17935 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 57190,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "20445 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2921,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "470762 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7692,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "154000 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 438939,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2677 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 23.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "48137188 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 366,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3301005 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 112624153,
            "unit": "ns/op\t36460804 B/op\t  817189 allocs/op",
            "extra": "12 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 6364,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "350324 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 759,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1574808 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1464,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "775659 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 993,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1216668 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 827,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1436738 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3486,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "365480 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 169,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "7288688 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12952,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "90880 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1819,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "666376 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 297,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3908329 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2321,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "510007 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 145257,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "7950 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 18520,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "58864 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 21190,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "58557 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 135610,
            "unit": "ns/op\t   20190 B/op\t     195 allocs/op",
            "extra": "9030 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6710,
            "unit": "ns/op\t    1994 B/op\t      46 allocs/op",
            "extra": "173014 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2567,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "451533 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 211,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5866920 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 23706,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "50997 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 163493,
            "unit": "ns/op\t  161630 B/op\t    1204 allocs/op",
            "extra": "6645 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 251,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "5119528 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4371,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "288183 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1836,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "669447 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 74182,
            "unit": "ns/op\t   37743 B/op\t     179 allocs/op",
            "extra": "15334 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1263,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "889417 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 327,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3670555 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 641,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1788910 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "144155518 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 206,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5778546 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 232,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5398588 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3053,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "376603 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1392,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "895864 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 405,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2989729 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1086,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 124,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "9588746 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 609,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1957396 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2236,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "511298 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 747,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1581967 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2630,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "444541 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1008,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 337,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3556234 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1211,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 332,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3567298 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1278,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "942745 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 239001,
            "unit": "ns/op\t   50053 B/op\t     191 allocs/op",
            "extra": "4705 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2724,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "406306 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 225,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5102858 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1921,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "626448 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 252,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4701620 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 856,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1394886 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 475,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2590394 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 28504856,
            "unit": "ns/op\t 5459618 B/op\t   80032 allocs/op",
            "extra": "37 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 169,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7003002 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4525,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "249418 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4652,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "255937 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 97.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "10648650 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1791,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "632502 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 250,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4583322 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 7710,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "147956 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6089,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "200944 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6062,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "190606 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3238,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "378580 times\n2 procs"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "blackb1rd.mov@gmail.com",
            "name": "Prachya Saechua",
            "username": "blackb1rd"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "015e9a9399f8b5306ab21c0866bc95b52080f5fc",
          "message": "Hugo v0.78.0 (#28)\n\n* Allow getJSON errors to be ignored\r\n\r\nThis change is mostly motivated to get a more stable CI build (we're building the Hugo site there, with Instagram and Twitter shortcodes sometimes failing).\r\n\r\nFixes #7866\r\n\r\n* Allow cascade _target to work with non toml fm\r\n\r\nThe TOML lib unmarshals slices of string maps to []map[string]interface{}\r\nwhereas YAML and JSON decode to []interface{}\r\n\r\nThe existing tests only check for TOML working correctly, and _target\r\nwith cascade did not work at all for frontmatter defined in other formats.\r\n\r\nAdd a function to normalize those slices\r\n\r\nFixes #7874\r\n\r\n* tpl: Fix reflection bug in merge\r\n\r\nValue.Type().Key() must only be called on map values.\r\n\r\nFixes #7899\r\n\r\n* Fix setting HUGO_MODULE_PROXY etc. via env vars\r\n\r\nFixes #7903\r\n\r\n* Squashed 'docs/' changes from 9abd3043a..d1157b687\r\n\r\nd1157b687 Fix typo -- missing word in title\r\n34c9a9ff3 Remove Hartwell Insurance subpage broken link\r\nd42f6d8eb Fix strings.Repeat documentation\r\n09b49debb Release 0.76.5\r\n149877735 Merge branch 'tempv0.76.5'\r\n72ffeb026 releaser: Add release notes to /docs for release of 0.76.5\r\n4150d8ae8 Fixed typo: update PrevPage to Prev\r\nc3e630db1 Update index.md\r\ndeb7520a8 Release 0.76.4\r\nf58bd7134 Merge branch 'tempv0.76.4'\r\n2581fe4bc releaser: Add release notes to /docs for release of 0.76.4\r\n26d8417fc Release 0.76.3\r\n34c49e06a Merge branch 'tempv0.76.3'\r\n0de2af2ef releaser: Add release notes to /docs for release of 0.76.3\r\ncce12c0f7 Add missing closing quotes and fix code-block lang (#1250)\r\n8c1c80d9f Update AMP link (#1251)\r\n7b1211ffa Fix typos in multilingual.md\r\n29e9d70fd Update index.md\r\n172a5480b Release 0.76.2\r\n93ff424d3 Merge branch 'temp762'\r\n3cad3c23a releaser: Add release notes to /docs for release of 0.76.2\r\n610221964 Update index.md\r\n8a9d31709 Release 0.76.1\r\n64b245aa4 Merge branch 'temp761'\r\nf19469ff8 releaser: Add release notes to /docs for release of 0.76.1\r\n97f0ed030 Update front-matter.md\r\ncca71f263 Update index.md\r\n23c64f6a3 Fix typo in 0.76.0 release note\r\n5f79e034d Update index.md\r\nb66567de2 netlify: Bump to Hugo 0.76.0\r\na3e30300d Release 0.76.0\r\n3b057e8fe releaser: Add release notes to /docs for release of 0.76.0\r\n7fa1cd912 docs: Regen docshelper\r\n769712aec Merge commit 'e5568488051a571df48401e03f1304b95dbc9028'\r\n70ce6ae33 pagemeta: Make BuildConfig.Render an enum\r\na78d3849f Allow cascade to be a slice with a _target discriminator\r\n705fea656 Add force flag to server redirects config\r\n04e4e1cc1 tpl: Add Do Not Track (dnt) option to Vimeo shortcode\r\n025c19fe1 Fix CLI example for PostCSS 8\r\n0c70c47b2 markup/asciidocext: Add preserveTOC option\r\n\r\ngit-subtree-dir: docs\r\ngit-subtree-split: d1157b687e25054501c3bcbb735da51229f66b74\r\n\r\n* docs: Regen docs helper\r\n\r\n* releaser: Add release notes for 0.77.0\r\n\r\nRename to *-ready.md to continue.\r\n[ci skip]\r\n\r\n* Update 0.77.0-relnotes.md\r\n\r\n[ci skip]\r\n\r\n* Release 0.77.0\r\n\r\n* releaser: Bump versions for release of 0.77.0\r\n\r\n[ci skip]\r\n\r\n* releaser: Prepare repository for 0.78.0-DEV\r\n\r\n[ci skip]\r\n\r\n* js.Build: Generate tsconfig files\r\n\r\nUpdates #7777\r\n\r\nAdded support to allow SourceMap files to be external to the build.\r\nIn addition added more information when the js compilation has an error.\r\nCorrectly append sourceMappingURL to output file.\r\nFix merge conflict.\r\n\r\n* Make js.Build fully support modules\r\n\r\nFixes #7816\r\nFixes #7777\r\nFixes #7916\r\n\r\n* js: Add avoidTDZ option\r\n\r\nFixes #7865\r\n\r\n* Squashed 'docs/' changes from d1157b687..57c1d1a67\r\n\r\n57c1d1a67 Add note about issue with Instagram shortcode\r\n2a4355113 Add example for regional language tags (#1139)\r\n5e9a60fc0 Format dates with \"th\", \"nd\", and \"rd\". (#1254)\r\n43df9a7f6 Update index.md\r\nf15f32590 Update index.md\r\n5a15be93d Update configuration.md\r\ne71c1c545 Update configuration.md\r\n5fd0439ff Release Hugo 0.77.0\r\n5b875477c releaser: Add release notes to /docs for release of 0.77.0\r\n09a2a3199 docs: Regen docs helper\r\nbebef9bbe Merge commit '9cabb46f68bae01aeb1859727dcb21e8a10f5ec7'\r\nf5ed02685 tpl: Refactor time.AsTime location implementation\r\nf3b8eccd2 tpl: Update Hugo time to support optional [LOCATION] parameter\r\n\r\ngit-subtree-dir: docs\r\ngit-subtree-split: 57c1d1a67b9da5ba8ad5151d464f3fd7a21a24d8\r\n\r\n* releaser: Add release notes for 0.78.0\r\n\r\nRename to *-ready.md to continue.\r\n[ci skip]\r\n\r\n* Update 0.78.0-relnotes.md\r\n\r\n* releaser: Bump versions for release of 0.78.0\r\n\r\n[ci skip]\r\n\r\n* releaser: Add release notes to /docs for release of 0.78.0\r\n\r\n[ci skip]\r\n\r\n* releaser: Prepare repository for 0.79.0-DEV\r\n\r\n[ci skip]\r\n\r\n* fix: golintformat\r\n\r\n* fix: test\r\n\r\n* fix: deprecating set-env and add-path and fix name\r\n\r\nCo-authored-by: Bjørn Erik Pedersen <bjorn.erik.pedersen@gmail.com>\r\nCo-authored-by: Gareth Watts <gareth@omnipotent.net>\r\nCo-authored-by: Cameron Moore <moorereason@gmail.com>\r\nCo-authored-by: hugoreleaser <bjorn.erik.pedersen+hugoreleaser@gmail.com>\r\nCo-authored-by: Andreas Richter <andy@selfstudy.com>",
          "timestamp": "2020-11-03T23:10:33+07:00",
          "tree_id": "b1cf06cc1f5bd680d6e2fc5a478943896d8490bc",
          "url": "https://github.com/neohugo/neohugo/commit/015e9a9399f8b5306ab21c0866bc95b52080f5fc"
        },
        "date": 1604420479823,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 31.6,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "34604835 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 25.6,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "46235742 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4725,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "232921 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 100,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "12537688 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 11031,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "119928 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 11244,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "93050 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 59577,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "19590 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 56683,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "20266 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 4393,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "519374 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7428,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "165618 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 468636,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2502 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 24.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "49904330 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 387,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3219370 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 114243439,
            "unit": "ns/op\t35289705 B/op\t  779461 allocs/op",
            "extra": "12 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 31628,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "423518 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 763,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1512385 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1465,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "814111 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 947,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1255399 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 831,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1450881 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3502,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "323028 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 168,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "7289121 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12980,
            "unit": "ns/op\t    6334 B/op\t     112 allocs/op",
            "extra": "92953 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1766,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "726308 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 292,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4104300 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2294,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "518737 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 148659,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "7573 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 18320,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "63694 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 21469,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "55046 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 139978,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "9009 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6571,
            "unit": "ns/op\t    2000 B/op\t      46 allocs/op",
            "extra": "164257 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2724,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "464488 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 219,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5446311 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 24297,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "50180 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 167465,
            "unit": "ns/op\t  161631 B/op\t    1204 allocs/op",
            "extra": "6300 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 259,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4815357 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4448,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "270838 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1875,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "677799 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 80792,
            "unit": "ns/op\t   37742 B/op\t     179 allocs/op",
            "extra": "15229 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1268,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "904140 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 330,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3588074 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 650,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1855927 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 9.15,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "129668148 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 205,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5730014 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 232,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5169810 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3078,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "392820 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1440,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "837416 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 412,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2832240 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1085,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 129,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "8559280 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 631,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1913306 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2389,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "502467 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 762,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1559253 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2760,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "445405 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1026,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 358,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3447060 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1257,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "935018 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 350,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3432468 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1316,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "928168 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 250534,
            "unit": "ns/op\t   50066 B/op\t     191 allocs/op",
            "extra": "4984 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2653,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "459710 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 228,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4813470 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1969,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "571132 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 255,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4773919 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 847,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1370661 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 473,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2541590 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 28325602,
            "unit": "ns/op\t 5459300 B/op\t   80032 allocs/op",
            "extra": "42 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 182,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "5874295 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4596,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "259736 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4831,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "237043 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 85.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "13398758 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1823,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "603543 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 260,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4665768 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8131,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "148405 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6395,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "191004 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6292,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "196129 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3359,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "369805 times\n2 procs"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "blackb1rd.mov@gmail.com",
            "name": "Prachya Saechua",
            "username": "blackb1rd"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c9b796fa217e5206cd7fe87100ec1b161ec264cf",
          "message": "Fix/custom render hooks(AMP format) (#11)\n\n* hugolib: Fix Output format (AMP) render hooks is not map correctly\r\n\r\n* hugolib: Fix Custom Output format render hooks is not map correctly\r\n\r\n* fix: custom output format render hook\r\n\r\n* hugolib: Fix Custom Output format render hooks is not map correctly\r\n\r\n* Revert \"hugolib: Fix Custom Output format render hooks is not map correctly\"\r\n\r\nThis reverts commit 344269dff3eadf84917e3cd9cbc8d21c1de64dbb.",
          "timestamp": "2020-11-04T23:04:25+07:00",
          "tree_id": "ad0a6d385dda75330b94ce6e572a0eb0ae9fbe28",
          "url": "https://github.com/neohugo/neohugo/commit/c9b796fa217e5206cd7fe87100ec1b161ec264cf"
        },
        "date": 1604506567738,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 21.9,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "47121848 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 21.9,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "55422771 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 3738,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "344230 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 74.2,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "17037198 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 7927,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "131470 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 9140,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "127543 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 46382,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "26965 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 42751,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "27925 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 9908,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "664767 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 5752,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "177394 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 351577,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "3105 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 20,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "52323847 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 279,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "4303623 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 79999004,
            "unit": "ns/op\t35379574 B/op\t  784299 allocs/op",
            "extra": "16 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 185888,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "528196 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 543,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2203653 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1172,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "903710 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 714,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1686288 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 619,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "2031267 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 2760,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "450163 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 108,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "11008506 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 9864,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "122420 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1371,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "839965 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 229,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5334925 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 1767,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "677337 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 112384,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "11112 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 13651,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "87855 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 15856,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "77439 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 101801,
            "unit": "ns/op\t   20190 B/op\t     195 allocs/op",
            "extra": "11917 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 4722,
            "unit": "ns/op\t    1970 B/op\t      46 allocs/op",
            "extra": "221761 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 1990,
            "unit": "ns/op\t     503 B/op\t       2 allocs/op",
            "extra": "601562 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 167,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "7293814 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 17692,
            "unit": "ns/op\t    1344 B/op\t     120 allocs/op",
            "extra": "67744 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 119183,
            "unit": "ns/op\t  161622 B/op\t    1204 allocs/op",
            "extra": "10484 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 204,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "6056684 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 3366,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "357128 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1443,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "821320 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 52612,
            "unit": "ns/op\t   37739 B/op\t     179 allocs/op",
            "extra": "21961 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 884,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1320448 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 225,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5210857 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 467,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "2614146 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 6.41,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "185342550 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 149,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "8267762 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 163,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "7007089 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 2433,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "531190 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1044,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 305,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3975247 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 821,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1461810 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 89.7,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "13376020 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 492,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "2472724 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 1822,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "688898 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 583,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "2095950 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2015,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "624838 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 748,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1554264 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 254,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4951032 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 914,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1310223 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 241,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4910900 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 936,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1292718 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 140947,
            "unit": "ns/op\t   50066 B/op\t     191 allocs/op",
            "extra": "8014 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2067,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "631102 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 181,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "6678787 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1536,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "755719 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 207,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5659826 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 665,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1829835 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 353,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "3226610 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 22327661,
            "unit": "ns/op\t 5458934 B/op\t   80032 allocs/op",
            "extra": "50 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 126,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "8639017 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 3518,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "348499 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 3729,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "333840 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 118,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "11205602 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1427,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "730615 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 189,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "5696130 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 6068,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "202375 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 4617,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "263439 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 4589,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "271278 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 2466,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "514790 times\n2 procs"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "blackb1rd.mov@gmail.com",
            "name": "Prachya Saechua",
            "username": "blackb1rd"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6b95c8cd8bfd38e60dd6eeb6c4197aaa2ddbadba",
          "message": "Merge pull request #39 from blackb1rd/hugo/v0.78.1\n\nHugo: v0.78.1",
          "timestamp": "2020-11-05T19:58:15+07:00",
          "tree_id": "3bf1fa5e1602a15df7e520b9337085e1b4e391d7",
          "url": "https://github.com/neohugo/neohugo/commit/6b95c8cd8bfd38e60dd6eeb6c4197aaa2ddbadba"
        },
        "date": 1604581715453,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 33.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "30061681 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 26.6,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "49161312 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 5114,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "221823 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 98.2,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "13258024 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10963,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "107977 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 12492,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "105877 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 61699,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "18882 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 59708,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "20227 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2784,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "462102 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7712,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "159583 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 478833,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2427 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 24.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "49231903 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 392,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3086835 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 120137819,
            "unit": "ns/op\t36040761 B/op\t  802728 allocs/op",
            "extra": "10 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 28742,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "422713 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 782,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1560232 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1492,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "784129 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 1018,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1234761 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 859,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1388679 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3615,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "327603 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 172,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "7017921 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 13767,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "88720 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1757,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "675987 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 299,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4026456 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2307,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "487278 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 158152,
            "unit": "ns/op\t  118573 B/op\t     916 allocs/op",
            "extra": "7706 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 19531,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "61222 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 22938,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "54088 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 142428,
            "unit": "ns/op\t   20189 B/op\t     195 allocs/op",
            "extra": "8469 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 7014,
            "unit": "ns/op\t    2005 B/op\t      46 allocs/op",
            "extra": "157803 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2793,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "441260 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 221,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5199711 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 25549,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "48144 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 177487,
            "unit": "ns/op\t  161629 B/op\t    1204 allocs/op",
            "extra": "6283 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 278,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4510912 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4532,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "265332 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1932,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "661779 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 84982,
            "unit": "ns/op\t   37752 B/op\t     179 allocs/op",
            "extra": "14192 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1322,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "810000 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 337,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3532681 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 657,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1793449 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 9.09,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "132821875 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 213,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5413540 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 235,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5062062 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3143,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "370546 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1545,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "838762 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 425,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2716230 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1110,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 129,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "9231351 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 654,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1855184 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2414,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "492558 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 784,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1502088 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2809,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "438961 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1063,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 361,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3311001 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1324,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "920329 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 400,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "2935269 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1348,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "878652 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 259659,
            "unit": "ns/op\t   50072 B/op\t     191 allocs/op",
            "extra": "5016 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2752,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "447708 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 242,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4790424 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 2011,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "599470 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 263,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4600399 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 877,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1293621 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 498,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2455726 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 29447838,
            "unit": "ns/op\t 5459589 B/op\t   80032 allocs/op",
            "extra": "37 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 181,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "6494884 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4758,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "243955 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 5108,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "257216 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 92,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "13579509 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1910,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "586242 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 272,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4492650 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8664,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "139816 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6672,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "189829 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6533,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "191216 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3457,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "362686 times\n2 procs"
          }
        ]
      },
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
          "id": "dc6233a51e19987028b2656db9a38dcb2b20e71f",
          "message": "fix: merge hugo v0.78.1",
          "timestamp": "2020-11-05T20:10:11+07:00",
          "tree_id": "73b2e7d0b4dcf171b15f4a32b51cdf657420ae38",
          "url": "https://github.com/neohugo/neohugo/commit/dc6233a51e19987028b2656db9a38dcb2b20e71f"
        },
        "date": 1604582476549,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 27.4,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "45205258 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 31.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "45230646 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4703,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "236691 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 92.1,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "11241734 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10909,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "114482 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 12905,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "81506 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 57964,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "20678 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 52692,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "22060 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2533,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "520670 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7504,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "154166 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 455178,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2341 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 26.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "45172608 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 356,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3423962 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 109715646,
            "unit": "ns/op\t36638965 B/op\t  824147 allocs/op",
            "extra": "12 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 143563,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "435570 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 714,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1673078 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1460,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "805915 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 900,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1337443 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 813,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1519416 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3376,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "364292 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 140,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8332832 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12703,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "93974 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1723,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "635470 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 287,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4005559 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2175,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "539298 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 138646,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "7698 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17615,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "65818 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 20087,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "60824 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 133856,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "8994 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6532,
            "unit": "ns/op\t    1994 B/op\t      46 allocs/op",
            "extra": "171658 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2538,
            "unit": "ns/op\t     505 B/op\t       3 allocs/op",
            "extra": "483646 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 220,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5502573 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 22622,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "52550 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 154251,
            "unit": "ns/op\t  161631 B/op\t    1204 allocs/op",
            "extra": "6784 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 259,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4475323 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4255,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "286942 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1938,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "681789 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 68084,
            "unit": "ns/op\t   37722 B/op\t     179 allocs/op",
            "extra": "17090 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1142,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "975912 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 295,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4005248 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 609,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1999306 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.85,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "127141340 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 200,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5686159 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 216,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5519350 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3049,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "397245 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1362,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "921850 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 385,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3146437 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1059,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "998668 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 120,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "9392245 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 646,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1928602 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2331,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "544744 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 750,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1648608 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2619,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "453334 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 983,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1227451 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 319,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3609691 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1249,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "993420 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 359,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3187096 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1241,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "814044 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 188761,
            "unit": "ns/op\t   50062 B/op\t     191 allocs/op",
            "extra": "5533 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2619,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "476815 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 235,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5158933 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1983,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "655635 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 271,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4369999 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 861,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1364731 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 479,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2628807 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 29205333,
            "unit": "ns/op\t 5459435 B/op\t   80032 allocs/op",
            "extra": "39 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 172,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7148931 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4621,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "267891 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4990,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "225392 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 128,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "11472657 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1949,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "586516 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 253,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4720927 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 7999,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "150913 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6081,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "196788 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6034,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "204627 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3321,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "384660 times\n2 procs"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "blackb1rd.mov@gmail.com",
            "name": "Prachya Saechua",
            "username": "blackb1rd"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "28694976fc282271910f2a3f4604f8eddd40f7df",
          "message": "Merge pull request #43 from blackb1rd/setting/DisableHugoGeneratorInject\n\nchange meta inject for hugo generator default to true",
          "timestamp": "2020-11-06T21:31:51+07:00",
          "tree_id": "f6a03c6b66f0b8645808bc980603dcec1e173d06",
          "url": "https://github.com/neohugo/neohugo/commit/28694976fc282271910f2a3f4604f8eddd40f7df"
        },
        "date": 1604673732538,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 29.2,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "46359202 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 28.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "45302565 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4908,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "258715 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 93.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "12039572 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10654,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "128068 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 12981,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "80811 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 53333,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "21584 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 49964,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "23068 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 4323,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "570859 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7111,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "177306 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 440827,
            "unit": "ns/op\t  103265 B/op\t    2220 allocs/op",
            "extra": "2472 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 25.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "45043204 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 359,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3417022 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 109012390,
            "unit": "ns/op\t37016803 B/op\t  837727 allocs/op",
            "extra": "13 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 35335,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "428874 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 717,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1724954 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1441,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "918433 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 890,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1342959 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 785,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1553487 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3303,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "355768 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 136,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8597260 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12803,
            "unit": "ns/op\t    6337 B/op\t     112 allocs/op",
            "extra": "91929 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1710,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "731736 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 295,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4303436 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2332,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "539608 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 143524,
            "unit": "ns/op\t  118573 B/op\t     916 allocs/op",
            "extra": "7059 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 16869,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "67154 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 19557,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "61640 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 128750,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "9267 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6155,
            "unit": "ns/op\t    1974 B/op\t      46 allocs/op",
            "extra": "211381 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2395,
            "unit": "ns/op\t     503 B/op\t       2 allocs/op",
            "extra": "486409 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 198,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "6187192 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 21944,
            "unit": "ns/op\t    1344 B/op\t     120 allocs/op",
            "extra": "57409 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 151828,
            "unit": "ns/op\t  161633 B/op\t    1204 allocs/op",
            "extra": "7266 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 251,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4646534 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4150,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "286352 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1894,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "727693 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 71524,
            "unit": "ns/op\t   37708 B/op\t     179 allocs/op",
            "extra": "16748 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1143,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "990420 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 291,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4018231 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 612,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1932783 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 9.03,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "144199968 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 193,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6115324 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 217,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5750450 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3085,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "398263 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1424,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "935284 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 379,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3004408 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1035,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 114,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "10632817 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 625,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1936962 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2230,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "528081 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 711,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1670826 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2555,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "478881 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 984,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1227208 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 316,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3781772 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1162,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1049326 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 342,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3443409 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1216,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "931068 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 176778,
            "unit": "ns/op\t   50055 B/op\t     191 allocs/op",
            "extra": "6517 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2460,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "520515 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 234,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5241296 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1991,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "651081 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 266,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4321596 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 836,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1367169 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 440,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2759504 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 27748354,
            "unit": "ns/op\t 5459300 B/op\t   80032 allocs/op",
            "extra": "42 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 154,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7536074 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4317,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "276154 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4714,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "254442 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 134,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8583006 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1846,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "581592 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 244,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4952978 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 7759,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "153670 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6052,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "200883 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 5861,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "203961 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3216,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "368780 times\n2 procs"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "blackb1rd.mov@gmail.com",
            "name": "Prachya Saechua",
            "username": "blackb1rd"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c5d30dbb632434d6452392882a9ab3408ee55c77",
          "message": "Merge pull request #44 from blackb1rd/fix/merge_v0.78.1\n\nfix: merge hugo v0.78.1",
          "timestamp": "2020-11-06T22:42:10+07:00",
          "tree_id": "67afb5a96497e7b7dbce63ca63736ca63570e14b",
          "url": "https://github.com/neohugo/neohugo/commit/c5d30dbb632434d6452392882a9ab3408ee55c77"
        },
        "date": 1604677941875,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 33,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "35522605 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 27.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "43567676 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4998,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "206540 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 107,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "12017821 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 7642,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "157557 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 11396,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "107089 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 65705,
            "unit": "ns/op\t   31425 B/op\t      71 allocs/op",
            "extra": "18012 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 59865,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "19783 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2940,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "459747 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7732,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "136266 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 477476,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2448 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 25.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "42941623 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 396,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3075652 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 115779179,
            "unit": "ns/op\t35692081 B/op\t  794714 allocs/op",
            "extra": "12 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 12626,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "392013 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 777,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1537525 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1542,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "750261 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 1002,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1205008 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 865,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1396044 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3630,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "342877 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 179,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "6940432 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 13455,
            "unit": "ns/op\t    6335 B/op\t     112 allocs/op",
            "extra": "89600 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1860,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "708711 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 316,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3791379 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2551,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "480698 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 158060,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "7227 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 19097,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "62143 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 22671,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "55165 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 145404,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "8601 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6750,
            "unit": "ns/op\t    2004 B/op\t      46 allocs/op",
            "extra": "157754 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2751,
            "unit": "ns/op\t     503 B/op\t       2 allocs/op",
            "extra": "442519 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 229,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5217608 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 24940,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "48226 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 171604,
            "unit": "ns/op\t  161624 B/op\t    1204 allocs/op",
            "extra": "6319 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 269,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4441213 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4639,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "258463 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1961,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "624715 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 79189,
            "unit": "ns/op\t   37726 B/op\t     179 allocs/op",
            "extra": "14984 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1279,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "834114 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 335,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3616262 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 668,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1802892 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 9.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "127619368 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 218,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5438084 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 246,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "4898814 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3322,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "383950 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1483,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "842943 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 429,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2845060 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1134,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 135,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "9193047 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 665,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1861196 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2394,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "512340 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 782,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1521948 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2772,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "435870 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1043,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 365,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3331572 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1307,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "912074 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 412,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "2973758 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1367,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "876428 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 248594,
            "unit": "ns/op\t   50068 B/op\t     191 allocs/op",
            "extra": "4783 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2801,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "446478 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 240,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4990290 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 2058,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "593284 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 272,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4499467 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 903,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1348686 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 500,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2444338 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 29889722,
            "unit": "ns/op\t 5459516 B/op\t   80032 allocs/op",
            "extra": "38 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 179,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "6831394 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4911,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "253504 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 5029,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "247640 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 111,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "11781465 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1941,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "626121 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 272,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4243819 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8577,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "139677 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6737,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "187815 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6447,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "191692 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3461,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "345876 times\n2 procs"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "blackb1rd.mov@gmail.com",
            "name": "Prachya Saechua",
            "username": "blackb1rd"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2aa6fc52d5a54a7f131900c80c909a78ac93ea2d",
          "message": "tpl: Add urlencode, urldecode function (#41)\n\n* tpl: Add urls.Encode, urls.Decode function\r\n\r\n* fix: add test and docs",
          "timestamp": "2020-11-07T02:14:00+07:00",
          "tree_id": "14f1f8fe461ffe49fa005b1c05e4c71e2edc1cf7",
          "url": "https://github.com/neohugo/neohugo/commit/2aa6fc52d5a54a7f131900c80c909a78ac93ea2d"
        },
        "date": 1604690717959,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 27.1,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "44551890 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 30.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "45092331 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 5047,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "237973 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 96.1,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "13015336 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 11026,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "111003 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 13088,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "94765 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 57389,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "20572 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 54136,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "21928 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 3199,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "551680 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7495,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "159040 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 377041,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "3261 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 21.2,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "51931203 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 371,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3304339 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 86151868,
            "unit": "ns/op\t36513300 B/op\t  816035 allocs/op",
            "extra": "16 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 186393,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "483396 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 574,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2151104 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1228,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "963950 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 721,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1671487 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 674,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1940722 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 2918,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "434590 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 114,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "10045404 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 10906,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "117124 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1454,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "869913 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 232,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5217600 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 1856,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "562124 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 111966,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "10728 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 14030,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "91143 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 16645,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "73258 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 110588,
            "unit": "ns/op\t   20189 B/op\t     195 allocs/op",
            "extra": "11686 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 4885,
            "unit": "ns/op\t    1975 B/op\t      46 allocs/op",
            "extra": "212035 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2116,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "580586 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 177,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "6408733 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 18639,
            "unit": "ns/op\t    1344 B/op\t     120 allocs/op",
            "extra": "65486 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 126440,
            "unit": "ns/op\t  161622 B/op\t    1204 allocs/op",
            "extra": "9187 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 206,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "6036550 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 3546,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "331860 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1542,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "880484 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 54847,
            "unit": "ns/op\t   37723 B/op\t     179 allocs/op",
            "extra": "22897 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 928,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1258110 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 236,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4992043 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 490,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "2521804 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 7.04,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "170812962 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 161,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "7503024 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 187,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "6822410 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 2530,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "491414 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1083,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 316,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3625178 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 824,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1430575 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 92.1,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "12074427 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 514,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "2346973 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 1812,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "600726 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 592,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "2035734 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2051,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "631893 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 813,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1516933 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 258,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4620642 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 949,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1259014 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 289,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4250070 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 967,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1267556 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 147604,
            "unit": "ns/op\t   50055 B/op\t     191 allocs/op",
            "extra": "8278 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2184,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "483198 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 194,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "6172176 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1617,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "678348 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 215,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5271534 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 688,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1736682 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 362,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "3241717 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 22846340,
            "unit": "ns/op\t 5458985 B/op\t   80032 allocs/op",
            "extra": "49 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 129,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "9501908 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 3665,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "328678 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 3756,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "307006 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 123,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "10447330 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1473,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "755947 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 208,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "5718488 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 6507,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "178911 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 4843,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "244191 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 4714,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "246290 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 2631,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "464365 times\n2 procs"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "blackb1rd.mov@gmail.com",
            "name": "Prachya Saechua",
            "username": "blackb1rd"
          },
          "committer": {
            "email": "blackb1rd.mov@gmail.com",
            "name": "blackb1rd",
            "username": "blackb1rd"
          },
          "distinct": true,
          "id": "54e2344e2405bf5945277bf0d4ecd9722d177f25",
          "message": "ci: add automatic deploy doc (#46)",
          "timestamp": "2020-11-07T23:01:02+07:00",
          "tree_id": "7c462b38f6b39fca710b67c4ebfa399a12ed7a1f",
          "url": "https://github.com/neohugo/neohugo/commit/54e2344e2405bf5945277bf0d4ecd9722d177f25"
        },
        "date": 1604765468512,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 30.6,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "33080913 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 25.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "48939225 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4737,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "276074 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 98,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "13598058 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 9636,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "130270 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 10089,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "110095 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 62686,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "19791 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 56485,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "21520 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2854,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "388946 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7504,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "172604 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 448568,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2370 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 22.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "53507606 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 363,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3226209 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 112168084,
            "unit": "ns/op\t36326632 B/op\t  813294 allocs/op",
            "extra": "12 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 30209,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "428264 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 712,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1722114 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1501,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "914438 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 953,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1245099 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 815,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1531665 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3405,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "333874 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 156,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "7410462 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12217,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "96387 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1746,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "743978 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 290,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4012954 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2502,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "477319 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 142035,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "7968 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 18086,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "65260 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 24485,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "52986 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 137178,
            "unit": "ns/op\t   20187 B/op\t     195 allocs/op",
            "extra": "9523 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6038,
            "unit": "ns/op\t    1986 B/op\t      46 allocs/op",
            "extra": "186919 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2530,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "466821 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 210,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5753481 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 23225,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "51986 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 159665,
            "unit": "ns/op\t  161624 B/op\t    1204 allocs/op",
            "extra": "6697 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 242,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "5037142 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4271,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "275044 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1778,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "704973 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 76182,
            "unit": "ns/op\t   37739 B/op\t     179 allocs/op",
            "extra": "15658 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1188,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 301,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3959734 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 611,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1926472 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.48,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "133682121 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 199,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6079718 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 221,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5387157 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 2960,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "425011 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1355,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "835863 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 389,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2991189 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1076,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 121,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "10451605 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 594,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "2024104 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2382,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "449794 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 769,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1583473 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2640,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "431216 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1021,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 329,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3306864 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1187,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 331,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3712904 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1226,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "977463 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 227569,
            "unit": "ns/op\t   50062 B/op\t     191 allocs/op",
            "extra": "5265 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2564,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "481358 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 235,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5447125 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1853,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "665941 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 243,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4865829 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 818,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1473252 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 442,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2670174 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 26825388,
            "unit": "ns/op\t 5459243 B/op\t   80032 allocs/op",
            "extra": "43 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 164,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "6827912 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4563,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "270549 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4598,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "266406 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 106,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "9982872 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1771,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "612878 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 252,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4869108 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 7812,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "150812 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6127,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "209354 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 5889,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "213943 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3194,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "352946 times\n2 procs"
          }
        ]
      },
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
          "id": "a180354b841a22776ba86d58604b74a05b2b486f",
          "message": "ci: fix ci",
          "timestamp": "2020-11-08T00:06:04+07:00",
          "tree_id": "bdd2d8d696bb32706401e46c076d4d0b33ee8975",
          "url": "https://github.com/neohugo/neohugo/commit/a180354b841a22776ba86d58604b74a05b2b486f"
        },
        "date": 1604769456951,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 26.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "46102221 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 29.4,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "48108298 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4427,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "300094 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 94.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "13154136 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10329,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "131390 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 11212,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "93373 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 52964,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "22485 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 51095,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "21405 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 5330,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "549717 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 6855,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "173720 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 422201,
            "unit": "ns/op\t  103266 B/op\t    2220 allocs/op",
            "extra": "2691 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 25.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "46918299 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 346,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3478317 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 98874059,
            "unit": "ns/op\t35653603 B/op\t  787021 allocs/op",
            "extra": "14 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 176201,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "507315 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 676,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1777915 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1396,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "886272 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 852,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1392283 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 746,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1603390 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3219,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "363309 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 134,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8965917 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 11565,
            "unit": "ns/op\t    6337 B/op\t     112 allocs/op",
            "extra": "104503 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1618,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "728229 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 276,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4391679 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2170,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "587637 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 132554,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "8074 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 16345,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "75124 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 19156,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "63831 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 127794,
            "unit": "ns/op\t   20187 B/op\t     195 allocs/op",
            "extra": "9171 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 5922,
            "unit": "ns/op\t    1983 B/op\t      46 allocs/op",
            "extra": "194212 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2427,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "500497 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 205,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5850184 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 21738,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "56004 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 142865,
            "unit": "ns/op\t  161628 B/op\t    1204 allocs/op",
            "extra": "7432 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 247,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4868562 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4035,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "303949 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1787,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "675381 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 61823,
            "unit": "ns/op\t   37711 B/op\t     179 allocs/op",
            "extra": "18909 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1126,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1077224 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 293,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4303152 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 571,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "2149957 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.33,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "148882372 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 183,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6667308 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 215,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "6038767 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 2987,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "401341 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1281,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "942226 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 371,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3225673 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1005,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 109,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "10402975 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 612,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1997437 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2124,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "588530 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 690,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1708436 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2527,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "510524 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 953,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1233075 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 309,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3768669 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1161,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "874172 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 301,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3864717 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1162,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1034068 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 178386,
            "unit": "ns/op\t   50055 B/op\t     191 allocs/op",
            "extra": "6679 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2602,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "502042 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 228,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5481732 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1837,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "636246 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 258,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4697940 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 807,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1474275 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 446,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2740812 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 27473984,
            "unit": "ns/op\t 5459308 B/op\t   80032 allocs/op",
            "extra": "42 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 156,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7834702 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4357,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "277854 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4533,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "256620 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 133,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "9404365 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1768,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "685486 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 238,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "5180763 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 7479,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "156230 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 5736,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "203103 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 5773,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "211330 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3026,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "398785 times\n2 procs"
          }
        ]
      }
    ]
  }
}