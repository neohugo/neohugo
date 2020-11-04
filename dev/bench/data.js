window.BENCHMARK_DATA = {
  "lastUpdate": 1604506568605,
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
      }
    ]
  }
}