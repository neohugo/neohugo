window.BENCHMARK_DATA = {
  "lastUpdate": 1608005531872,
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
          "id": "d7579043e59049e6aa015521e8e3d4a4a21272ec",
          "message": "docs: update docs (#47)",
          "timestamp": "2020-11-08T01:47:51+07:00",
          "tree_id": "7ac2f273547106894be4c9154a95d74dafed94be",
          "url": "https://github.com/neohugo/neohugo/commit/d7579043e59049e6aa015521e8e3d4a4a21272ec"
        },
        "date": 1604775557554,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 24.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "44682086 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 24.1,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "43891410 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4055,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "284499 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 86.4,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "14630595 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 6443,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "188338 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 10532,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "112183 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 49417,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "23630 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 48231,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "24386 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 4888,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "582828 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 6564,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "160000 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 397605,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "3000 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 23.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "48490437 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 320,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3795048 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 91169823,
            "unit": "ns/op\t35615830 B/op\t  790004 allocs/op",
            "extra": "14 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 205002,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "504602 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 661,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1827139 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1366,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "901351 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 907,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1370268 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 775,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1521864 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3397,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "358395 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 141,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8548866 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12160,
            "unit": "ns/op\t    6337 B/op\t     112 allocs/op",
            "extra": "97388 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1889,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "541675 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 284,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4160242 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2190,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "509118 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 140205,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "8827 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17306,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "71530 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 19925,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "61812 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 136247,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "9279 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6372,
            "unit": "ns/op\t    1993 B/op\t      46 allocs/op",
            "extra": "177280 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2688,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "464636 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 216,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5536646 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 22858,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "51934 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 151239,
            "unit": "ns/op\t  161628 B/op\t    1204 allocs/op",
            "extra": "7845 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 263,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4629626 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4284,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "283321 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1847,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "707113 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 69260,
            "unit": "ns/op\t   37734 B/op\t     179 allocs/op",
            "extra": "17031 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1165,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1029439 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 297,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4089343 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 619,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1991136 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.31,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "136599157 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 199,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6068028 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 213,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5658012 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3112,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "396849 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1356,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "896944 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 388,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3053678 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1041,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 115,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "10301726 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 638,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1913960 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2284,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "521382 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 717,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1652692 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2573,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "461532 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1001,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 321,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3671436 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1175,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 314,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3768090 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1212,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 185732,
            "unit": "ns/op\t   50060 B/op\t     191 allocs/op",
            "extra": "5913 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2606,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "479001 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 231,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5262289 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1947,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "636985 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 267,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4330065 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 868,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1375690 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 466,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2649238 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 29047215,
            "unit": "ns/op\t 5459422 B/op\t   80032 allocs/op",
            "extra": "40 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 164,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7075413 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4628,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "264356 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4673,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "262436 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 124,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "9244088 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1869,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "592456 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 253,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4922271 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8006,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "139994 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6155,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "196603 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6021,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "204321 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3293,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "372876 times\n2 procs"
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
          "id": "e64c4e8b42d4f57935f519eb4385c0cf6c4f24df",
          "message": "fix bench script (#49)",
          "timestamp": "2020-11-08T23:56:34+07:00",
          "tree_id": "e9b9bbf865870c09fe8d4b5bbc85da4f3f781233",
          "url": "https://github.com/neohugo/neohugo/commit/e64c4e8b42d4f57935f519eb4385c0cf6c4f24df"
        },
        "date": 1604855287705,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 28.6,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "45615916 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 29.6,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "38693404 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4839,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "241611 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 90.9,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "12267571 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 7689,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "145304 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 12102,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "99021 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 55072,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "20517 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 52012,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "22333 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2460,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "530581 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7684,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "158895 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 454407,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2502 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 26,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "41472343 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 363,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3250268 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 107187113,
            "unit": "ns/op\t35701554 B/op\t  793411 allocs/op",
            "extra": "13 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 227859,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "464175 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 738,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1612165 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1491,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "693842 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 914,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1292595 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 824,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1458987 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3495,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "361443 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 146,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8404047 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12861,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "87186 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1779,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "678190 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 298,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4017974 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2241,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "473672 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 144453,
            "unit": "ns/op\t  118573 B/op\t     916 allocs/op",
            "extra": "7898 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17939,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "67075 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 20967,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "60235 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 136715,
            "unit": "ns/op\t   20187 B/op\t     195 allocs/op",
            "extra": "9007 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6574,
            "unit": "ns/op\t    1999 B/op\t      46 allocs/op",
            "extra": "165015 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2606,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "471434 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 234,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5382164 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 23299,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "50828 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 159322,
            "unit": "ns/op\t  161628 B/op\t    1204 allocs/op",
            "extra": "7633 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 266,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4539776 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4443,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "276170 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1962,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "678643 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 72657,
            "unit": "ns/op\t   37739 B/op\t     179 allocs/op",
            "extra": "17390 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1179,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "937824 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 306,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3744877 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 620,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1917024 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.66,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "131428486 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 201,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5994178 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 222,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5414930 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3163,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "385563 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1371,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "880284 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 397,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3058570 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1059,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 117,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "10416136 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 649,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1865083 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2324,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "501640 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 755,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1593302 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2716,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "401181 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1014,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 341,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3537294 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1214,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "996702 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 341,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3540232 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1258,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "980806 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 197837,
            "unit": "ns/op\t   50060 B/op\t     191 allocs/op",
            "extra": "6171 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2715,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "426174 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 238,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5017603 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1990,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "609631 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 278,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4218160 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 885,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1360128 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 477,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2536310 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 30246670,
            "unit": "ns/op\t 5459434 B/op\t   80032 allocs/op",
            "extra": "40 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 166,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7416889 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4652,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "246421 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4873,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "248757 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 129,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "10364268 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1945,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "572846 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 250,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4712254 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8009,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "158089 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6437,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "184935 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6124,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "195771 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3269,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "370755 times\n2 procs"
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
          "id": "4531f469b87212d6734b8429773ab9422f3bab02",
          "message": "Fix: optimize memory (#51)\n\n* fix bench script\r\n\r\n* Avoid defer and remove workaround since its was fixed in upsteam\r\n\r\n* fix: check after lock before process\r\n\r\n* fix: avoid defer\r\n\r\n* fix: update golang.org/x/net\r\n\r\n* fix: wrtiestat map key was overwrite\r\n\r\n* fix: format",
          "timestamp": "2020-11-12T00:15:55+07:00",
          "tree_id": "6ca1fd6c81cfdcbca31f673e6ae2c86a1e318d65",
          "url": "https://github.com/neohugo/neohugo/commit/4531f469b87212d6734b8429773ab9422f3bab02"
        },
        "date": 1605115673242,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 30.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "45512607 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 28.4,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "45551481 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4821,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "242998 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 85.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "13486075 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10459,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "96241 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 11054,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "100231 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 54888,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "21310 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 52233,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "22497 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 3416,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "583857 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7465,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "161630 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 449294,
            "unit": "ns/op\t  103265 B/op\t    2220 allocs/op",
            "extra": "2559 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 26.1,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "44996011 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 358,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3394574 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 102783476,
            "unit": "ns/op\t36511450 B/op\t  814283 allocs/op",
            "extra": "13 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 231776,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "493666 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 702,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1689159 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1455,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "832394 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 889,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1329308 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 785,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1547443 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3331,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "359287 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 140,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8471223 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12580,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "93850 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1682,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "691020 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 285,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4208798 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2146,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "550485 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 139053,
            "unit": "ns/op\t  118573 B/op\t     916 allocs/op",
            "extra": "7706 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17207,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "69226 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 20117,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "58822 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 132729,
            "unit": "ns/op\t   20187 B/op\t     195 allocs/op",
            "extra": "9115 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6296,
            "unit": "ns/op\t    1987 B/op\t      46 allocs/op",
            "extra": "183120 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2566,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "474741 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 218,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5521710 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 22485,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "52081 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 147526,
            "unit": "ns/op\t  161628 B/op\t    1204 allocs/op",
            "extra": "7621 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 255,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4748512 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4178,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "283676 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1820,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "710121 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 65924,
            "unit": "ns/op\t   37718 B/op\t     179 allocs/op",
            "extra": "17811 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1153,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1040799 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 288,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4097496 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 618,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1959645 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.23,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "132999387 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 194,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6324747 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 213,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5664973 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3043,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "398456 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1310,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "906367 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 372,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3137112 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1032,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 115,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "10584204 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 614,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1915731 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2233,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "556468 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 737,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1659703 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2587,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "478263 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1013,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 327,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3668446 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1157,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "972879 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 347,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3414012 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1238,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 186564,
            "unit": "ns/op\t   50064 B/op\t     191 allocs/op",
            "extra": "6345 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2550,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "479485 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 227,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5089308 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1893,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "659971 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 267,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4518417 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 853,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1392895 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 469,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2587725 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 28947268,
            "unit": "ns/op\t 5459514 B/op\t   80032 allocs/op",
            "extra": "38 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 161,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7510800 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4563,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "270584 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4628,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "264799 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 136,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "10349228 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1848,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "639211 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 253,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4874164 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8048,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "150021 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6111,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "194246 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6057,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "206536 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3269,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "371730 times\n2 procs"
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
          "id": "e9aebf37058860cfce13a0007fd3ac57bb16dc89",
          "message": "hugo: v0.78.2 (#52)\n\n* build(deps): bump github.com/evanw/esbuild from 0.8.3 to 0.8.4\r\n\r\nBumps [github.com/evanw/esbuild](https://github.com/evanw/esbuild) from 0.8.3 to 0.8.4.\r\n- [Release notes](https://github.com/evanw/esbuild/releases)\r\n- [Changelog](https://github.com/evanw/esbuild/blob/master/CHANGELOG.md)\r\n- [Commits](https://github.com/evanw/esbuild/compare/v0.8.3...v0.8.4)\r\n\r\nSigned-off-by: dependabot[bot] <support@github.com>\r\n\r\n* build(deps): bump golang.org/x/text from 0.3.3 to 0.3.4\r\n\r\nBumps [golang.org/x/text](https://github.com/golang/text) from 0.3.3 to 0.3.4.\r\n- [Release notes](https://github.com/golang/text/releases)\r\n- [Commits](https://github.com/golang/text/compare/v0.3.3...v0.3.4)\r\n\r\nSigned-off-by: dependabot[bot] <support@github.com>\r\n\r\n* build(deps): bump github.com/frankban/quicktest from 1.11.1 to 1.11.2\r\n\r\nBumps [github.com/frankban/quicktest](https://github.com/frankban/quicktest) from 1.11.1 to 1.11.2.\r\n- [Release notes](https://github.com/frankban/quicktest/releases)\r\n- [Commits](https://github.com/frankban/quicktest/compare/v1.11.1...v1.11.2)\r\n\r\nSigned-off-by: dependabot[bot] <support@github.com>\r\n\r\n* Added first fuzzer\r\n\r\n* Updated year in header\r\n\r\n* Update GH docs to say \"main\" as default branch\r\n\r\n* build(deps): bump github.com/getkin/kin-openapi from 0.22.1 to 0.26.0\r\n\r\nBumps [github.com/getkin/kin-openapi](https://github.com/getkin/kin-openapi) from 0.22.1 to 0.26.0.\r\n- [Release notes](https://github.com/getkin/kin-openapi/releases)\r\n- [Commits](https://github.com/getkin/kin-openapi/compare/v0.22.1...v0.26.0)\r\n\r\nSigned-off-by: dependabot[bot] <support@github.com>\r\n\r\n* build(deps): bump github.com/evanw/esbuild from 0.8.4 to 0.8.5\r\n\r\nBumps [github.com/evanw/esbuild](https://github.com/evanw/esbuild) from 0.8.4 to 0.8.5.\r\n- [Release notes](https://github.com/evanw/esbuild/releases)\r\n- [Changelog](https://github.com/evanw/esbuild/blob/master/CHANGELOG.md)\r\n- [Commits](https://github.com/evanw/esbuild/compare/v0.8.4...v0.8.5)\r\n\r\nSigned-off-by: dependabot[bot] <support@github.com>\r\n\r\n* build(deps): bump github.com/evanw/esbuild from 0.8.5 to 0.8.6\r\n\r\nBumps [github.com/evanw/esbuild](https://github.com/evanw/esbuild) from 0.8.5 to 0.8.6.\r\n- [Release notes](https://github.com/evanw/esbuild/releases)\r\n- [Changelog](https://github.com/evanw/esbuild/blob/master/CHANGELOG.md)\r\n- [Commits](https://github.com/evanw/esbuild/compare/v0.8.5...v0.8.6)\r\n\r\nSigned-off-by: dependabot[bot] <support@github.com>\r\n\r\n* js: Let ESBuild handle all imports from node_modules\r\n\r\nThis commit fixes some issues where modules in /assets share the same name as in node_modules.\r\n\r\nThis was not intended, and with this commit the node_modules-components should be isolated. If you want to redefine something inside node_modules, use the `defines` option.\r\n\r\nFixes #7948\r\n\r\n* releaser: Add release notes for 0.78.2\r\n[ci skip]\r\n\r\n* releaser: Bump versions for release of 0.78.2\r\n\r\n[ci skip]\r\n\r\n* releaser: Add release notes to /docs for release of 0.78.2\r\n\r\n[ci skip]\r\n\r\n* releaser: Prepare repository for 0.79.0-DEV\r\n\r\n[ci skip]\r\n\r\n* hugo: v0.78.2\r\n\r\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>\r\nCo-authored-by: AdamKorcz <adam@adalogics.com>\r\nCo-authored-by: Mackenzie Morgan <macoafi@gmail.com>\r\nCo-authored-by: Bjørn Erik Pedersen <bjorn.erik.pedersen@gmail.com>\r\nCo-authored-by: hugoreleaser <bjorn.erik.pedersen+hugoreleaser@gmail.com>",
          "timestamp": "2020-11-13T22:09:20+07:00",
          "tree_id": "e7b547ab9c968534196d4ad0ec0e92b69890b92c",
          "url": "https://github.com/neohugo/neohugo/commit/e9aebf37058860cfce13a0007fd3ac57bb16dc89"
        },
        "date": 1605280879187,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 29.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "41773711 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 30,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "42270828 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4897,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "247015 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 84.1,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "14507834 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 7341,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "143887 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 10341,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "115977 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 56473,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "20661 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 54477,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "21787 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 4701,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "560091 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7602,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "163154 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 487432,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2448 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 26.9,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "44517708 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 394,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3243495 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 111587399,
            "unit": "ns/op\t36094904 B/op\t  805263 allocs/op",
            "extra": "12 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 239450,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "452842 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 722,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1660644 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1487,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "828962 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 932,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1288662 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 809,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1462275 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3563,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "317530 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 143,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8386647 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 13266,
            "unit": "ns/op\t    6335 B/op\t     112 allocs/op",
            "extra": "90591 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1700,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "712779 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 290,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3975212 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2195,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "543015 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 148772,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "7005 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 18629,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "64053 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 21026,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "52831 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 139717,
            "unit": "ns/op\t   20187 B/op\t     195 allocs/op",
            "extra": "8929 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6561,
            "unit": "ns/op\t    1996 B/op\t      46 allocs/op",
            "extra": "168452 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2632,
            "unit": "ns/op\t     505 B/op\t       3 allocs/op",
            "extra": "467205 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 224,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5403145 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 23370,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "51429 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 159522,
            "unit": "ns/op\t  161625 B/op\t    1204 allocs/op",
            "extra": "7414 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 267,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4462831 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4343,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "278101 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1963,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "697921 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 75653,
            "unit": "ns/op\t   37726 B/op\t     179 allocs/op",
            "extra": "15828 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1214,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1016869 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 311,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3810135 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 644,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1932446 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.46,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "141644548 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 202,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5905006 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 222,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5406171 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3276,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "378844 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1419,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "879259 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 397,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2991434 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1093,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "978940 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 121,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "9436921 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 658,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1880230 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2318,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "531398 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 755,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1587643 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2712,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "453477 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1016,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 328,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3643018 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1216,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "976201 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 319,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3558172 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1283,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "984676 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 208273,
            "unit": "ns/op\t   50064 B/op\t     191 allocs/op",
            "extra": "6042 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2728,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "452353 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 249,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4902594 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 2007,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "619114 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 278,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4059148 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 893,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1371435 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 486,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2448945 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 30141682,
            "unit": "ns/op\t 5459575 B/op\t   80032 allocs/op",
            "extra": "37 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 172,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "6695652 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4842,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "249654 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4985,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "246632 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 123,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "9844414 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1966,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "543490 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 260,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4640656 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 7988,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "134316 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6254,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "196335 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6205,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "201312 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3341,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "367676 times\n2 procs"
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
          "id": "5e511b7f191ee9a8773771bdece068e2fd1f8200",
          "message": "Revert \"hugo: v0.78.2 (#52)\" (#53)\n\nThis reverts commit e9aebf37058860cfce13a0007fd3ac57bb16dc89.",
          "timestamp": "2020-11-13T22:24:36+07:00",
          "tree_id": "6ca1fd6c81cfdcbca31f673e6ae2c86a1e318d65",
          "url": "https://github.com/neohugo/neohugo/commit/5e511b7f191ee9a8773771bdece068e2fd1f8200"
        },
        "date": 1605281773574,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 29.1,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "42979416 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 30.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "44488158 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4891,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "235290 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 85,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "12380983 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10747,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "110629 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 11039,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "102265 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 56837,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "20784 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 55694,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "20992 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2508,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "417441 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7591,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "165108 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 484218,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2536 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 28.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "44218308 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 370,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3346386 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 110586338,
            "unit": "ns/op\t37057779 B/op\t  836568 allocs/op",
            "extra": "13 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 204290,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "486392 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 744,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1647864 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1529,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "761812 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 940,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1257822 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 820,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1443768 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3626,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "321178 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 143,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8404183 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 13725,
            "unit": "ns/op\t    6335 B/op\t     112 allocs/op",
            "extra": "91994 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1689,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "694416 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 290,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4144546 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2182,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "530520 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 147095,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "7383 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 18101,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "65086 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 21694,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "52610 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 138800,
            "unit": "ns/op\t   20187 B/op\t     195 allocs/op",
            "extra": "8802 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6625,
            "unit": "ns/op\t    1998 B/op\t      46 allocs/op",
            "extra": "168102 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2599,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "472328 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 224,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5353486 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 23202,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "50930 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 158394,
            "unit": "ns/op\t  161621 B/op\t    1204 allocs/op",
            "extra": "6850 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 269,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4415148 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4353,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "279813 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1858,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "693672 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 77530,
            "unit": "ns/op\t   37747 B/op\t     179 allocs/op",
            "extra": "15045 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1231,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "957625 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 307,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3813580 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 632,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1854552 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.51,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "129458008 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 212,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5802670 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 220,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5246383 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3186,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "373738 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1400,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "804178 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 391,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3037020 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1071,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 119,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "9791550 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 644,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1888141 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2312,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "514078 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 744,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1605218 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2669,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "461541 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1044,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 327,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3658462 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1242,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "930542 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 360,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3055686 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1233,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "978596 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 196894,
            "unit": "ns/op\t   50067 B/op\t     191 allocs/op",
            "extra": "5946 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2738,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "459037 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 245,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4937748 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 2003,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "621846 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 277,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4302242 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 879,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1347997 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 489,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2545353 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 29728209,
            "unit": "ns/op\t 5459452 B/op\t   80032 allocs/op",
            "extra": "39 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 170,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7094860 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4899,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "253215 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4792,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "260246 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 114,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "11130470 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1940,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "573613 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 259,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4515534 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8068,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "138222 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6463,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "189822 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6170,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "193926 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3345,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "350160 times\n2 procs"
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
          "id": "9ee104da5770792d9c578c3a7a1de12afaa7879b",
          "message": "Merge pull request #54 from blackb1rd/hugo/v0.78.2\n\nHugo: v0.78.2",
          "timestamp": "2020-11-13T22:25:28+07:00",
          "tree_id": "e7b547ab9c968534196d4ad0ec0e92b69890b92c",
          "url": "https://github.com/neohugo/neohugo/commit/9ee104da5770792d9c578c3a7a1de12afaa7879b"
        },
        "date": 1605281841300,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 27.6,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "43201998 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 26.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "42371919 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4760,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "312069 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 85.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "12951394 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10510,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "134917 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 9961,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "103947 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 54633,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "21549 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 51848,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "22368 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2333,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "579429 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7393,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "169982 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 444507,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2529 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 26.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "43833404 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 354,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3415083 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 104123164,
            "unit": "ns/op\t36549947 B/op\t  819221 allocs/op",
            "extra": "13 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 231767,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "475917 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 707,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1674405 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1436,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "833439 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 885,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1350244 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 790,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1555201 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3342,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "385501 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 139,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8626794 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12419,
            "unit": "ns/op\t    6337 B/op\t     112 allocs/op",
            "extra": "95383 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1700,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "681826 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 281,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4216252 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2195,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "532394 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 135019,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "7563 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 16866,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "70726 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 19601,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "61533 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 130170,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "9379 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6295,
            "unit": "ns/op\t    1986 B/op\t      46 allocs/op",
            "extra": "187042 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2488,
            "unit": "ns/op\t     503 B/op\t       3 allocs/op",
            "extra": "467055 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 215,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5659164 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 22550,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "53595 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 144528,
            "unit": "ns/op\t  161622 B/op\t    1204 allocs/op",
            "extra": "7154 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 252,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4782796 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4150,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "284325 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1808,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "772712 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 65169,
            "unit": "ns/op\t   37724 B/op\t     179 allocs/op",
            "extra": "18483 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1105,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 279,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4270198 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 575,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "2012434 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.07,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "141743839 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 190,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6424908 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 209,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5810550 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 2968,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "400810 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1324,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "919227 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 379,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3068859 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1009,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 111,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "10673896 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 609,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1967587 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2167,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "490867 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 712,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1679653 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2498,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "436897 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 963,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1232304 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 313,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3762490 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1124,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1047834 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 308,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3912424 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1187,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "893252 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 183701,
            "unit": "ns/op\t   50061 B/op\t     191 allocs/op",
            "extra": "6745 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2543,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "477242 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 229,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5270916 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1874,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "660692 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 260,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4632898 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 832,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1423062 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 441,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2669989 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 27400050,
            "unit": "ns/op\t 5459423 B/op\t   80032 allocs/op",
            "extra": "40 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 156,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7434397 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4499,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "265996 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4602,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "264968 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 132,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "9012180 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1807,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "628296 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 238,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "5080364 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 7663,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "155409 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 5939,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "200820 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 5862,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "213654 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3135,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "390308 times\n2 procs"
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
          "id": "18fe50fa9d425a9f5cc1ceda580deda94f38a8cc",
          "message": "Merge pull request #56 from blackb1rd/fix/remove_defer\n\nfix: remove defer",
          "timestamp": "2020-11-14T09:14:44+07:00",
          "tree_id": "6016f9cab048b00150c248b70f8a4cfef5c8df83",
          "url": "https://github.com/neohugo/neohugo/commit/18fe50fa9d425a9f5cc1ceda580deda94f38a8cc"
        },
        "date": 1605320778428,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 22.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "53320860 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 24.2,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "43858494 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4166,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "243608 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 85.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "14834419 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 9242,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "127275 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 8978,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "134023 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 48455,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "24483 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 46082,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "24728 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2866,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "610027 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 6322,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "190518 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 392931,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "3069 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 22.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "46545363 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 304,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "4006737 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 92308729,
            "unit": "ns/op\t34433288 B/op\t  749434 allocs/op",
            "extra": "14 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 239834,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "467620 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 741,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2010118 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1506,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "990304 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 835,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1525012 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 687,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1760283 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3010,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "414283 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 119,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "10088688 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 11515,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "95498 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1499,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "778042 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 241,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4972028 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 1859,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "621058 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 123456,
            "unit": "ns/op\t  118573 B/op\t     916 allocs/op",
            "extra": "8972 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 15555,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "76369 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 18093,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "62067 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 115415,
            "unit": "ns/op\t   20189 B/op\t     195 allocs/op",
            "extra": "10592 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 5569,
            "unit": "ns/op\t    1986 B/op\t      46 allocs/op",
            "extra": "186046 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2223,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "560155 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 185,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "6493298 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 20056,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "52916 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 145433,
            "unit": "ns/op\t  161630 B/op\t    1204 allocs/op",
            "extra": "8205 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 216,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "5597894 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 3649,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "330693 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1559,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "838004 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 62510,
            "unit": "ns/op\t   37705 B/op\t     179 allocs/op",
            "extra": "19360 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1003,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 247,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4815525 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 517,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "2157638 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 7.22,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "168182716 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 220,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "7092231 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 183,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "6475195 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 2999,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "448476 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1156,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "893815 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 324,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3701086 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 891,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1334619 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 98.6,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "12038107 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 523,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "2434360 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 1936,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "638415 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 629,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1885131 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2224,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "525040 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 818,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1412149 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 273,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4345852 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1035,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 270,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4443878 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1024,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 172722,
            "unit": "ns/op\t   50062 B/op\t     191 allocs/op",
            "extra": "7082 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2241,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "549554 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 208,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5786282 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1641,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "751728 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 247,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4957471 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 766,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1395196 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 421,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2888281 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 25559284,
            "unit": "ns/op\t 5459150 B/op\t   80032 allocs/op",
            "extra": "45 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 141,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "8501798 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4153,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "303274 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4261,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "286758 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 110,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "9594319 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1650,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "678030 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 220,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "5294362 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 6825,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "174664 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 5324,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "226420 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 5243,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "236666 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 2865,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "429391 times\n2 procs"
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
          "id": "57d7f94b4239632fc6e70273ecadea090794b72e",
          "message": "ci: fix go version (#57)",
          "timestamp": "2020-11-14T09:51:55+07:00",
          "tree_id": "0858374d4f923a57c7e66deaaab87b73746d622f",
          "url": "https://github.com/neohugo/neohugo/commit/57d7f94b4239632fc6e70273ecadea090794b72e"
        },
        "date": 1605322999550,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 29.1,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "43195719 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 28.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "38433682 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 5285,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "237355 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 84.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "14542585 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10966,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "120837 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 10304,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "116124 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 57091,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "20710 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 54522,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "21412 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 3812,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "526321 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7572,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "166407 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 457474,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2454 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 26.9,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "41848815 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 363,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3362647 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 110329895,
            "unit": "ns/op\t36761228 B/op\t  827855 allocs/op",
            "extra": "14 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 192449,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "440475 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 729,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1656626 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1478,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "819862 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 962,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1257712 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 814,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1467361 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3603,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "289048 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 146,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8084260 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 13332,
            "unit": "ns/op\t    6335 B/op\t     112 allocs/op",
            "extra": "92161 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1731,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "678312 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 292,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4040128 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2180,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "551584 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 150927,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "7075 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17879,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "66694 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 21498,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "55088 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 137552,
            "unit": "ns/op\t   20186 B/op\t     195 allocs/op",
            "extra": "8835 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6639,
            "unit": "ns/op\t    1999 B/op\t      46 allocs/op",
            "extra": "165836 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2574,
            "unit": "ns/op\t     503 B/op\t       3 allocs/op",
            "extra": "457095 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 222,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5417566 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 23949,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "49795 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 157494,
            "unit": "ns/op\t  161636 B/op\t    1204 allocs/op",
            "extra": "6740 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 255,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4618694 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4354,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "277095 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1870,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "692714 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 68744,
            "unit": "ns/op\t   37720 B/op\t     179 allocs/op",
            "extra": "16678 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1164,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "958134 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 305,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4000772 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 629,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1955671 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.98,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "141637081 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 205,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5932393 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 216,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5602725 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3167,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "387921 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1375,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "870715 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 392,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3088568 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1069,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 120,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "9590611 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 644,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1856059 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2332,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "517996 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 734,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1589944 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2647,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "462198 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1005,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 329,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3621993 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1225,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "999397 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 359,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3333388 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1238,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "966202 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 194943,
            "unit": "ns/op\t   50064 B/op\t     191 allocs/op",
            "extra": "6250 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2718,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "460939 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 244,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5028177 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1985,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "635362 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 271,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4299469 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 908,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1378364 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 478,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2531695 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 30257982,
            "unit": "ns/op\t 5459586 B/op\t   80032 allocs/op",
            "extra": "37 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 175,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7314222 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4729,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "255193 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4822,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "248883 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 161,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8333325 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1921,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "527508 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 260,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4545595 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8065,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "145890 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6309,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "199375 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6205,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "194648 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3345,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "346753 times\n2 procs"
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
          "id": "409557c0bedaa458da65bbd4a3b4708301f8dcf6",
          "message": "ci: add gotip (#58)\n\n* ci: add gotip\r\n\r\n* ci: fix gotip\r\n\r\n* ci: cleanup\r\n\r\n* ci: fix\r\n\r\n* ci: fix\r\n\r\n* Update golangci-lint.yml",
          "timestamp": "2020-11-15T01:57:15+07:00",
          "tree_id": "12dd36848ecd4571de1bc28a6154e43475ba52c6",
          "url": "https://github.com/neohugo/neohugo/commit/409557c0bedaa458da65bbd4a3b4708301f8dcf6"
        },
        "date": 1605380924160,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 22.9,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "59405548 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 20.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "59645920 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 3772,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "331609 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 73.2,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "16655269 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 6494,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "177475 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 8596,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "137710 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 45340,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "26113 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 41397,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "27867 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 21788,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "727238 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 5606,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "210991 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 336667,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "3200 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 20.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "60109684 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 276,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "4378141 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 82725781,
            "unit": "ns/op\t36105366 B/op\t  807050 allocs/op",
            "extra": "16 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 209699,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "475753 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 678,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1768083 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1405,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "894420 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 871,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1387687 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 777,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1568125 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3303,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "362628 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 129,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8834432 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 11822,
            "unit": "ns/op\t    6335 B/op\t     112 allocs/op",
            "extra": "101516 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1688,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "709857 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 276,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4416170 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2050,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "557673 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 135804,
            "unit": "ns/op\t  118573 B/op\t     916 allocs/op",
            "extra": "7884 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 16408,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "71546 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 19108,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "59154 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 125667,
            "unit": "ns/op\t   20189 B/op\t     195 allocs/op",
            "extra": "9717 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 5892,
            "unit": "ns/op\t    1979 B/op\t      46 allocs/op",
            "extra": "199417 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2434,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "486801 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 201,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "6067020 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 21678,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "55497 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 147469,
            "unit": "ns/op\t  161629 B/op\t    1204 allocs/op",
            "extra": "7207 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 232,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "5109028 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4031,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "303439 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1796,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "711760 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 63959,
            "unit": "ns/op\t   37710 B/op\t     179 allocs/op",
            "extra": "18385 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1072,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1179108 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 290,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4160821 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 569,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "2084802 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 7.6,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "150723616 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 184,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6533863 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 202,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5998840 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 2837,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "417500 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1273,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 436,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2757404 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1029,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 112,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "10626603 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 605,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1981658 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2194,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "560450 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 693,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1710390 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2415,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "497620 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 955,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1280836 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 315,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3801019 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1105,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 300,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4027166 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1170,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "919286 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 177158,
            "unit": "ns/op\t   50061 B/op\t     191 allocs/op",
            "extra": "6570 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2433,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "499358 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 214,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5537266 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1792,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "673420 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 245,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5004866 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 815,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1492341 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 430,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2796388 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 27021218,
            "unit": "ns/op\t 5459207 B/op\t   80032 allocs/op",
            "extra": "44 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 154,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "8057786 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4281,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "282876 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4532,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "284628 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 127,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "10347140 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1750,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "600370 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 236,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "5249748 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 7442,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "160116 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 5812,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "204426 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 5686,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "216402 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3069,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "396403 times\n2 procs"
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
          "id": "cda6f1258eee6c9154eb1ec311f733c524b0cfbb",
          "message": "ci: change container and upgrade golang (#60)",
          "timestamp": "2020-11-18T18:24:42+07:00",
          "tree_id": "33e88134b5d18cbbaafbd3a7be1dc91b93dd52a2",
          "url": "https://github.com/neohugo/neohugo/commit/cda6f1258eee6c9154eb1ec311f733c524b0cfbb"
        },
        "date": 1605699282274,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 32.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "34822778 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 29.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "45006974 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4827,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "248594 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 104,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "12743272 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 11230,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "117015 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 11852,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "84774 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 66061,
            "unit": "ns/op\t   31425 B/op\t      71 allocs/op",
            "extra": "17732 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 58271,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "20151 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 3150,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "461835 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 8079,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "145627 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 475222,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2473 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 24.9,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "46532559 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 394,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3067424 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 123597424,
            "unit": "ns/op\t36335237 B/op\t  813064 allocs/op",
            "extra": "10 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 6119,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "360848 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 773,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1557906 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1517,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "769890 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 968,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1219173 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 838,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1455128 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3596,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "342548 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 172,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "7048968 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 13142,
            "unit": "ns/op\t    6335 B/op\t     112 allocs/op",
            "extra": "88422 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1852,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "662684 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 305,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3954153 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2494,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "415383 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 154173,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "7318 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 19029,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "63794 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 22890,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "53775 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 140605,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "8725 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6761,
            "unit": "ns/op\t    2003 B/op\t      46 allocs/op",
            "extra": "157842 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2696,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "444382 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 226,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5518551 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 24893,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "49580 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 170636,
            "unit": "ns/op\t  161632 B/op\t    1204 allocs/op",
            "extra": "6246 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 262,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4574242 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4565,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "260119 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1926,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "655612 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 83276,
            "unit": "ns/op\t   37728 B/op\t     179 allocs/op",
            "extra": "14730 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1386,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "907533 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 347,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3482233 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 697,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1770225 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 10.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "100000000 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 231,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5305227 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 257,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "4593084 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3289,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "359853 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1586,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "826131 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 429,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2714973 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1137,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 134,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "8981890 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 640,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1815746 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2360,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "505741 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 770,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1540464 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2706,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "445471 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1031,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 357,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3424120 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1284,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "898114 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 378,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3269653 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1313,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "884328 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 247138,
            "unit": "ns/op\t   50063 B/op\t     191 allocs/op",
            "extra": "4642 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2733,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "447484 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 235,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5034822 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 2054,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "614612 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 277,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4458106 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 889,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1341774 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 486,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2441800 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 29305618,
            "unit": "ns/op\t 5459499 B/op\t   80032 allocs/op",
            "extra": "39 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 175,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "6787417 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4787,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "248845 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4899,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "253306 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 110,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "15323662 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1916,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "577984 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 272,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4416351 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8401,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "142905 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6576,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "186333 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6479,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "185444 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3507,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "348100 times\n2 procs"
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
          "id": "81bf1d4c5e3fa9e993fabcbfb1e71e50daa9a5a9",
          "message": "ci: separate tag build (#61)",
          "timestamp": "2020-11-18T21:02:21+07:00",
          "tree_id": "db2cc61b675e2aa001c88bb178d4dfb963e79194",
          "url": "https://github.com/neohugo/neohugo/commit/81bf1d4c5e3fa9e993fabcbfb1e71e50daa9a5a9"
        },
        "date": 1605708759168,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 30,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "42665264 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 27.1,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "51660475 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4787,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "218176 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 96.2,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "13245468 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10972,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "109461 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 11140,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "92382 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 61794,
            "unit": "ns/op\t   31425 B/op\t      71 allocs/op",
            "extra": "18979 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 55578,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "20926 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2795,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "510993 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7525,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "156609 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 449800,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2631 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 23.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "50280627 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 370,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3396445 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 107671592,
            "unit": "ns/op\t35602282 B/op\t  786185 allocs/op",
            "extra": "12 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 26984,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "417757 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 709,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1654924 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1412,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "830726 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 895,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1351094 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 753,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1594934 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3376,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "367910 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 156,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "7232829 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 11967,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "97212 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1737,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "687778 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 276,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4243371 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2255,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "506914 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 141931,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "8397 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17738,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "67818 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 20541,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "57846 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 130263,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "9458 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6175,
            "unit": "ns/op\t    1991 B/op\t      46 allocs/op",
            "extra": "176511 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2616,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "414280 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 224,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5518108 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 23883,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "44218 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 164553,
            "unit": "ns/op\t  161632 B/op\t    1204 allocs/op",
            "extra": "6394 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 254,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4693321 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4158,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "288411 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1767,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "745087 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 75288,
            "unit": "ns/op\t   37736 B/op\t     179 allocs/op",
            "extra": "15792 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1137,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "979707 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 295,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3957259 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 608,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1950084 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.49,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "143095160 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 200,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6086523 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 226,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5332238 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 2907,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "346833 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1350,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "880473 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 397,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3140179 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1052,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 119,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "9279086 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 596,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "2090986 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2209,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "550384 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 704,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1689475 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2554,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "476638 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1028,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 333,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3551892 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1175,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1014909 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 314,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3742530 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1197,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "922640 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 219019,
            "unit": "ns/op\t   50064 B/op\t     191 allocs/op",
            "extra": "4770 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2497,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "482154 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 210,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5532482 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1827,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "701882 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 250,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4806502 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 809,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1467939 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 456,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2692670 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 27286441,
            "unit": "ns/op\t 5459285 B/op\t   80032 allocs/op",
            "extra": "42 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 162,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7459264 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4683,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "258306 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4590,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "286084 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 112,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "12097671 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1851,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "638858 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 281,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4592065 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8237,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "142542 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6264,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "190416 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 5939,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "212857 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3317,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "361258 times\n2 procs"
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
          "id": "ac703a8fdd0ef98e4b2a759125a8e69c84538eac",
          "message": "ci: add docker to build (#62)",
          "timestamp": "2020-11-18T21:48:15+07:00",
          "tree_id": "4d5953671398cc96e9bb0b46e232213ab2352594",
          "url": "https://github.com/neohugo/neohugo/commit/ac703a8fdd0ef98e4b2a759125a8e69c84538eac"
        },
        "date": 1605711489408,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 36.2,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "33223262 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 31.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "46683829 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 5265,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "207024 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 114,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8833040 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 11624,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "108117 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 12609,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "91737 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 66296,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "16717 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 62773,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "19399 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 3241,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "411272 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 8177,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "142394 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 493929,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2156 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 25.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "45573531 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 410,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "2992117 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 134405224,
            "unit": "ns/op\t35718801 B/op\t  792912 allocs/op",
            "extra": "10 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 5692,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "364219 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 766,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1558038 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1499,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "873495 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 991,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1200988 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 847,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1355550 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3671,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "327091 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 177,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "6651786 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 13094,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "90328 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1857,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "749797 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 303,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4086462 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2382,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "507324 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 154999,
            "unit": "ns/op\t  118574 B/op\t     916 allocs/op",
            "extra": "7893 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 19752,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "60390 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 23047,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "51350 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 159805,
            "unit": "ns/op\t   20189 B/op\t     195 allocs/op",
            "extra": "7612 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 7144,
            "unit": "ns/op\t    2008 B/op\t      46 allocs/op",
            "extra": "153434 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2837,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "456099 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 226,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5308686 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 25819,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "43885 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 180279,
            "unit": "ns/op\t  161619 B/op\t    1204 allocs/op",
            "extra": "5574 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 266,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4199326 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4958,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "249414 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 2067,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "605149 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 94565,
            "unit": "ns/op\t   37751 B/op\t     179 allocs/op",
            "extra": "12741 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1291,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "869824 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 347,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3349321 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 710,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1712990 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 9.76,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "126754005 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 228,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5255545 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 257,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "4772020 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3444,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "361111 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1562,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "854397 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 448,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2657072 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1243,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 148,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "8688962 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 670,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1803535 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2560,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "491931 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 826,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1544733 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2867,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "430374 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1120,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 369,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3211455 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1398,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "920557 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 391,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3034291 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1399,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "831028 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 263056,
            "unit": "ns/op\t   50066 B/op\t     191 allocs/op",
            "extra": "4634 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2910,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "411366 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 258,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4742659 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 2094,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "600052 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 287,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4086328 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 929,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1282476 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 517,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2228848 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 31457440,
            "unit": "ns/op\t 5459555 B/op\t   80032 allocs/op",
            "extra": "38 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 187,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "6679521 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 5060,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "247333 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 5353,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "203341 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 107,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "13922910 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 2088,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "523776 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 297,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "3814040 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8935,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "134992 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 7122,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "180505 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6971,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "164772 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3719,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "327472 times\n2 procs"
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
          "id": "6f1f7c42b820631a71ac88c08d69b380902ba1ea",
          "message": "ci: fix docker push (#63)\n\n* ci: fix docker push\r\n\r\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* remove tag\r\n\r\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* fix docker build",
          "timestamp": "2020-11-19T18:04:46+07:00",
          "tree_id": "bbefa7bdad25236bd2b928e1d87b0897db7c653d",
          "url": "https://github.com/neohugo/neohugo/commit/6f1f7c42b820631a71ac88c08d69b380902ba1ea"
        },
        "date": 1605784574684,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 30.6,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "37952079 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 29.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "40445516 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 5124,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "233313 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 98.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "12866830 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10758,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "131217 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 11943,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "104822 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 58347,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "20510 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 54202,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "21049 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 5161,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "561333 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7386,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "159465 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 453381,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2540 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 26.9,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "43467258 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 373,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "2758060 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 111582173,
            "unit": "ns/op\t37350840 B/op\t  847280 allocs/op",
            "extra": "13 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 204102,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "442507 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 734,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1647171 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1505,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "812145 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 917,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1276962 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 800,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1512444 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3473,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "338486 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 141,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8493376 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12723,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "93550 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1760,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "685992 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 288,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4164788 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2283,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "441194 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 168531,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "6261 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 20472,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "59594 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 24272,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "54979 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 144254,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "7693 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6598,
            "unit": "ns/op\t    1995 B/op\t      46 allocs/op",
            "extra": "171094 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2715,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "465784 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 220,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5440254 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 23172,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "50373 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 154625,
            "unit": "ns/op\t  161624 B/op\t    1204 allocs/op",
            "extra": "6926 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 249,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4852342 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4338,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "277788 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 2035,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "650976 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 71479,
            "unit": "ns/op\t   37737 B/op\t     179 allocs/op",
            "extra": "17158 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1159,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "953827 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 315,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3740223 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 605,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1905027 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.43,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "139807102 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 195,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5960620 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 222,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5432331 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3175,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "371618 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1321,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "888832 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 450,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2682990 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1079,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 126,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "9846864 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 660,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1824649 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2276,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "483010 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 764,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1578340 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2908,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "460288 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1063,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 346,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3461210 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1315,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "923964 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 336,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3493795 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1302,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "978612 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 224143,
            "unit": "ns/op\t   50051 B/op\t     191 allocs/op",
            "extra": "6170 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2841,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "381210 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 272,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4284170 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 2153,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "638312 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 300,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "3972097 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 987,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1242458 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 534,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2294452 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 32570299,
            "unit": "ns/op\t 5459980 B/op\t   80032 allocs/op",
            "extra": "32 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 184,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7257837 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4793,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "235341 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4777,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "267175 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 122,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "9917946 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1967,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "537058 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 258,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4626312 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8075,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "151627 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6448,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "187131 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6308,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "199490 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3300,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "346798 times\n2 procs"
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
          "id": "b22821667fab2428a82d293025dee63629e8320b",
          "message": "Ci: fix build docker (#64)\n\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* remove tag\r\n\r\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* fix docker build\r\n\r\n* fix docker build",
          "timestamp": "2020-11-19T19:56:30+07:00",
          "tree_id": "060d5fb692c96bbcbcfb6d32596aa2d27ce9d0dd",
          "url": "https://github.com/neohugo/neohugo/commit/b22821667fab2428a82d293025dee63629e8320b"
        },
        "date": 1605791274218,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 29.4,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "43250004 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 28.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "44407410 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4984,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "243987 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 101,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "12006130 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10746,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "108124 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 11553,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "94026 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 55658,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "21346 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 53949,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "21781 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 4239,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "552076 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7374,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "168667 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 449869,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2566 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 26.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "44727889 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 376,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3293451 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 101247031,
            "unit": "ns/op\t35272844 B/op\t  778666 allocs/op",
            "extra": "13 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 218300,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "443035 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 710,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1702912 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1490,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "835490 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 921,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1295049 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 800,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1493847 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3490,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "342670 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 140,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8009877 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12598,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "92811 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1714,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "657866 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 286,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3937809 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2358,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "522440 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 140556,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "7878 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17290,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "68574 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 20354,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "61323 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 136316,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "9151 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6373,
            "unit": "ns/op\t    1995 B/op\t      46 allocs/op",
            "extra": "172827 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2563,
            "unit": "ns/op\t     502 B/op\t       2 allocs/op",
            "extra": "473037 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 220,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5463480 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 22966,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "52496 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 154063,
            "unit": "ns/op\t  161634 B/op\t    1204 allocs/op",
            "extra": "7531 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 251,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4748712 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4317,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "275184 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1890,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "620221 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 65139,
            "unit": "ns/op\t   37714 B/op\t     179 allocs/op",
            "extra": "18547 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1135,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "983865 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 297,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4075140 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 613,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1957545 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.41,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "141833424 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 196,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6326029 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 219,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5537883 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3029,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "391125 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1341,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "914443 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 448,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2648292 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1093,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 119,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "10385109 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 638,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1871420 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2320,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "527535 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 743,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1667509 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2656,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "434904 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1018,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 346,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3366266 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1211,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 317,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3779766 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1238,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "956762 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 191756,
            "unit": "ns/op\t   50056 B/op\t     191 allocs/op",
            "extra": "6284 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2664,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "468523 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 234,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4890888 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1974,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "644218 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 270,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4462347 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 873,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1378963 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 467,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2622813 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 28804309,
            "unit": "ns/op\t 5459296 B/op\t   80032 allocs/op",
            "extra": "42 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 173,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7310990 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4593,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "264370 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4591,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "247665 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 128,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8395213 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1863,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "586212 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 253,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4654682 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 7891,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "151362 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6194,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "194596 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6003,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "207954 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3286,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "377114 times\n2 procs"
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
          "id": "670f2e53e98c0e7d62544f477cc175cc0e7305a8",
          "message": "fix url docker (#65)",
          "timestamp": "2020-11-19T21:43:04+07:00",
          "tree_id": "b278fe1249ae5b423f6774c22e6ea404ba01aa6f",
          "url": "https://github.com/neohugo/neohugo/commit/670f2e53e98c0e7d62544f477cc175cc0e7305a8"
        },
        "date": 1605797682811,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 29.6,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "38918568 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 29.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "40536703 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4824,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "218833 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 96.9,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "12700323 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10645,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "121352 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 13007,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "92510 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 56937,
            "unit": "ns/op\t   31425 B/op\t      71 allocs/op",
            "extra": "20592 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 54527,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "21644 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2464,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "530391 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7509,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "158830 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 458316,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2613 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 29.1,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "42329668 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 381,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3337557 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 110543542,
            "unit": "ns/op\t36191911 B/op\t  809275 allocs/op",
            "extra": "12 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 207954,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "457176 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 747,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1677070 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1529,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "827017 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 933,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1289211 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 815,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1479148 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3516,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "303232 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 142,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8404654 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12707,
            "unit": "ns/op\t    6337 B/op\t     112 allocs/op",
            "extra": "89120 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1770,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "693226 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 288,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4171603 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2270,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "544977 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 142452,
            "unit": "ns/op\t  118573 B/op\t     916 allocs/op",
            "extra": "8136 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17895,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "68112 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 20704,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "56160 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 139603,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "8874 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6705,
            "unit": "ns/op\t    1997 B/op\t      46 allocs/op",
            "extra": "168435 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2590,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "468321 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 220,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5456367 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 23244,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "50025 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 155686,
            "unit": "ns/op\t  161629 B/op\t    1204 allocs/op",
            "extra": "6724 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 253,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4613344 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4332,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "277826 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1893,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "657424 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 67915,
            "unit": "ns/op\t   37709 B/op\t     179 allocs/op",
            "extra": "17736 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1161,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 300,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3893955 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 644,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1954078 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.63,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "141792529 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 203,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5989354 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 212,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5630256 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3170,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "392062 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1341,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "787106 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 457,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2576913 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1083,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 123,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "10255849 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 648,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1889944 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2278,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "537086 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 738,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1520512 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2663,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "465253 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1003,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 346,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3294620 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1202,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "994789 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 336,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3459520 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1253,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "948159 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 194621,
            "unit": "ns/op\t   50061 B/op\t     191 allocs/op",
            "extra": "6100 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2649,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "465354 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 239,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4943659 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 2020,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "634599 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 284,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4317093 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 879,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1332382 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 482,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2495953 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 29685845,
            "unit": "ns/op\t 5459465 B/op\t   80032 allocs/op",
            "extra": "39 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 172,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7217331 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4681,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "257287 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4923,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "254456 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 113,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "9308431 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1942,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "595238 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 253,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4459622 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8354,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "140406 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6376,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "194001 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6206,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "192741 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3393,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "369380 times\n2 procs"
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
          "id": "5198c25dbb3e7f128bc6add4551eaefd98dc33c2",
          "message": "docker",
          "timestamp": "2020-11-19T21:51:55+07:00",
          "tree_id": "c89d2fda6745b7766544ce81e57d1d89fa7acce9",
          "url": "https://github.com/neohugo/neohugo/commit/5198c25dbb3e7f128bc6add4551eaefd98dc33c2"
        },
        "date": 1605798266174,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 29.9,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "39499962 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 29.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "42023713 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4967,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "242552 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 101,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "13168802 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 9593,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "126793 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 11452,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "104347 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 56026,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "20780 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 54333,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "21696 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2484,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "493224 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7528,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "168885 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 463523,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2668 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 27.9,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "43362325 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 365,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3340939 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 110304365,
            "unit": "ns/op\t37180208 B/op\t  840896 allocs/op",
            "extra": "13 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 181131,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "470726 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 707,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1679334 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1484,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "847468 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 910,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1289569 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 801,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1485796 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3461,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "349688 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 148,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "7922721 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12041,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "94652 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1682,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "730214 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 288,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4183730 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2239,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "530420 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 143113,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "8092 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17468,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "67077 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 20477,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "61058 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 136489,
            "unit": "ns/op\t   20187 B/op\t     195 allocs/op",
            "extra": "8824 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6596,
            "unit": "ns/op\t    1996 B/op\t      46 allocs/op",
            "extra": "170608 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2688,
            "unit": "ns/op\t     505 B/op\t       3 allocs/op",
            "extra": "473623 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 220,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5444818 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 23971,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "49724 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 161353,
            "unit": "ns/op\t  161624 B/op\t    1204 allocs/op",
            "extra": "7611 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 250,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4793445 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4484,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "274998 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 2056,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "677440 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 66874,
            "unit": "ns/op\t   37703 B/op\t     179 allocs/op",
            "extra": "18055 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1136,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "973732 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 297,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4040760 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 610,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1949848 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "137472694 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 194,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6007574 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 215,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5688282 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3082,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "359084 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1382,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "930997 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 461,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2582684 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1091,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 127,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "9726848 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 664,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1838434 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2268,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "517909 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 727,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1642672 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2732,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "462892 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1034,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 346,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3462200 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1215,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "999642 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 323,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3682500 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1210,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "963955 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 185741,
            "unit": "ns/op\t   50063 B/op\t     191 allocs/op",
            "extra": "6270 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2661,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "465264 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 233,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4993646 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1967,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "625021 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 269,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4556236 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 856,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1394348 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 470,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2611251 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 29327485,
            "unit": "ns/op\t 5459536 B/op\t   80032 allocs/op",
            "extra": "39 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 171,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "6993848 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4558,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "261490 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4767,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "252723 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 146,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "7952790 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1914,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "629986 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 257,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4694518 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8102,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "145304 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6246,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "195939 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6182,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "199860 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3292,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "369500 times\n2 procs"
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
          "id": "c9a21cf67881c85a601ce1736f0d5265345bfea9",
          "message": "ci: add docker to build (#62)",
          "timestamp": "2020-11-19T21:53:13+07:00",
          "tree_id": "c89d2fda6745b7766544ce81e57d1d89fa7acce9",
          "url": "https://github.com/neohugo/neohugo/commit/c9a21cf67881c85a601ce1736f0d5265345bfea9"
        },
        "date": 1605798315135,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 30.1,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "38499883 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 29.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "41053399 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4924,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "245323 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 98.4,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "12707882 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10590,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "105102 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 11681,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "97372 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 56130,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "20824 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 53614,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "22261 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 5479,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "563030 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7491,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "161112 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 437995,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2547 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 26.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "41213430 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 362,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3366723 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 103742023,
            "unit": "ns/op\t35812653 B/op\t  797402 allocs/op",
            "extra": "14 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 224937,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "468996 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 718,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1667502 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1472,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "820742 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 908,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1319048 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 808,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1502041 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3481,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "351810 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 142,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8503282 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12642,
            "unit": "ns/op\t    6337 B/op\t     112 allocs/op",
            "extra": "95427 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1695,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "667166 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 286,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4196252 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2238,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "541405 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 145850,
            "unit": "ns/op\t  118573 B/op\t     916 allocs/op",
            "extra": "7400 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17288,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "70113 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 20184,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "60682 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 133961,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "9121 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6424,
            "unit": "ns/op\t    1992 B/op\t      46 allocs/op",
            "extra": "177554 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2567,
            "unit": "ns/op\t     503 B/op\t       3 allocs/op",
            "extra": "462398 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 220,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5454421 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 23049,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "51004 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 157145,
            "unit": "ns/op\t  161626 B/op\t    1204 allocs/op",
            "extra": "6949 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 250,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4809032 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4356,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "276555 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 2003,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "626089 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 71246,
            "unit": "ns/op\t   37717 B/op\t     179 allocs/op",
            "extra": "17593 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1156,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 298,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3996286 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 606,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1962992 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.42,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "142239308 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 199,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5683514 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 213,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5525516 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3123,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "381686 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1361,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "896605 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 454,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2735947 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1049,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 116,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "10245825 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 634,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1872016 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2257,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "516074 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 715,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1623956 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2585,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "421777 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 996,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1205905 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 348,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3457006 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1184,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "992226 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 329,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3588754 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1217,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1001074 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 188881,
            "unit": "ns/op\t   50056 B/op\t     191 allocs/op",
            "extra": "6200 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2567,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "461452 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 226,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5342640 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1916,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "637903 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 260,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4505318 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 849,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1393164 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 474,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2603720 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 29112870,
            "unit": "ns/op\t 5459497 B/op\t   80032 allocs/op",
            "extra": "38 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 164,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7313329 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4687,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "261853 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4859,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "256632 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 138,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8563482 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1946,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "513900 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 258,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4729558 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8093,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "151698 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6369,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "195274 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6175,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "191272 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3284,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "336115 times\n2 procs"
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
          "id": "e21f4bd4d5c0c33be6fa24d2b1bf609f2a8fb930",
          "message": "Create FUNDING.yml",
          "timestamp": "2020-11-26T23:50:36+07:00",
          "tree_id": "f7421ef908ea080d5e9d1edb829a8a4fba4dbc56",
          "url": "https://github.com/neohugo/neohugo/commit/e21f4bd4d5c0c33be6fa24d2b1bf609f2a8fb930"
        },
        "date": 1606410041799,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 28.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "42051454 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 29.2,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "44394534 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 5081,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "230396 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 93.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "11130184 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10458,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "109903 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 12397,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "89598 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 56472,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "20295 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 55096,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "21472 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2595,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "503852 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7578,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "131997 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 453995,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2544 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 27.2,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "44467404 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 364,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3280537 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 112447646,
            "unit": "ns/op\t37268272 B/op\t  844803 allocs/op",
            "extra": "14 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 14651,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "395456 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 721,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1664384 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1489,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "861067 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 926,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1279378 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 799,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1483201 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3588,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "349054 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 142,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8509296 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12914,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "91101 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1739,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "698211 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 297,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4206175 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2293,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "552400 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 142608,
            "unit": "ns/op\t  118573 B/op\t     916 allocs/op",
            "extra": "7638 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17615,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "67689 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 20703,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "56469 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 137049,
            "unit": "ns/op\t   20187 B/op\t     195 allocs/op",
            "extra": "9021 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6607,
            "unit": "ns/op\t    1994 B/op\t      46 allocs/op",
            "extra": "173842 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2636,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "425061 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 227,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5155489 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 23865,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "50155 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 161327,
            "unit": "ns/op\t  161631 B/op\t    1204 allocs/op",
            "extra": "6556 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 254,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4177542 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4266,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "275059 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1888,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "658092 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 68466,
            "unit": "ns/op\t   37728 B/op\t     179 allocs/op",
            "extra": "17467 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1201,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "981464 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 314,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3994489 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 624,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1952925 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 9.15,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "135161971 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 197,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6052344 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 215,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5509225 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3138,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "367063 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1365,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "893294 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 469,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2611168 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1074,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1118847 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 121,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "9737838 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 664,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1812873 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2360,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "529989 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 736,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1596099 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2629,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "461956 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1037,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 363,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3241772 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1236,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "970016 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 337,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3486747 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1262,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "979456 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 196840,
            "unit": "ns/op\t   50065 B/op\t     191 allocs/op",
            "extra": "5886 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2742,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "453685 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 240,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5121883 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1957,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "631878 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 273,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4372622 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 890,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1333126 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 486,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2491980 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 30204256,
            "unit": "ns/op\t 5459501 B/op\t   80032 allocs/op",
            "extra": "38 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 172,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "6863743 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4789,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "261140 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4916,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "258375 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 114,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "9319167 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1973,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "542050 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 259,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4570354 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8364,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "147553 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6275,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "186610 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6191,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "202272 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3377,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "375968 times\n2 procs"
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
          "id": "747a3d6faf3e2ae6f4908ceead87d63058a70d07",
          "message": "Update FUNDING.yml",
          "timestamp": "2020-11-26T23:55:49+07:00",
          "tree_id": "f9a0eecc9a438cf32d6ea95095a0616ef9a2ad81",
          "url": "https://github.com/neohugo/neohugo/commit/747a3d6faf3e2ae6f4908ceead87d63058a70d07"
        },
        "date": 1606410359583,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 24,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "52996978 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 23.2,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "46912244 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4256,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "237928 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 98.4,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "10359103 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 9431,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "115543 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 10918,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "121012 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 50308,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "23574 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 47889,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "25047 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2665,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "561463 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 6335,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "200737 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 392484,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2984 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 22.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "51896354 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 307,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3954192 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 109221356,
            "unit": "ns/op\t36084628 B/op\t  810538 allocs/op",
            "extra": "12 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 22284,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "403806 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 599,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1998756 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1218,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 793,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1522149 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 668,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1766916 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3020,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "406855 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 119,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "10023927 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 11991,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "98564 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1586,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "723464 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 241,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4978534 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2115,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "509901 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 137544,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "7705 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17185,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "68296 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 20123,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "59538 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 115553,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "10671 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 5942,
            "unit": "ns/op\t    1993 B/op\t      46 allocs/op",
            "extra": "174274 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2375,
            "unit": "ns/op\t     503 B/op\t       2 allocs/op",
            "extra": "515317 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 184,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "6506012 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 20585,
            "unit": "ns/op\t    1344 B/op\t     120 allocs/op",
            "extra": "59600 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 151905,
            "unit": "ns/op\t  161633 B/op\t    1204 allocs/op",
            "extra": "7039 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 211,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "5681209 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 3246,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "369360 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1675,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "778578 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 74239,
            "unit": "ns/op\t   37724 B/op\t     179 allocs/op",
            "extra": "16210 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 993,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1207903 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 250,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4841324 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 527,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "2262841 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 7.36,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "168032030 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 179,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6491632 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 187,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "6427094 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 2776,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "434644 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1174,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 380,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3156865 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 910,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1319251 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 113,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "10609311 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 540,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "2230396 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 1990,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "616216 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 633,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1930677 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2251,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "536204 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 867,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1392571 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 290,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4070041 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1034,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 271,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4378622 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1056,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 175990,
            "unit": "ns/op\t   50060 B/op\t     191 allocs/op",
            "extra": "6650 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2230,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "535815 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 210,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5775313 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1645,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "744957 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 239,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5025554 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 748,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1614048 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 426,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2833651 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 26019891,
            "unit": "ns/op\t 5459189 B/op\t   80032 allocs/op",
            "extra": "44 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 150,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7940832 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4215,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "289550 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4692,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "262426 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 123,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "9909972 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1703,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "704660 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 245,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4816624 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 6702,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "174435 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 5395,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "228298 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 5085,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "226968 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 2876,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "418345 times\n2 procs"
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
          "id": "f381a1160d073484bafc6fc4cc7d756ce4ec3e9b",
          "message": "Merge pull request #69 from blackb1rd/hugo/v0.79.0\n\nHugo: v0.79.0",
          "timestamp": "2020-11-27T21:24:47+07:00",
          "tree_id": "2a0a9fbd8ef6a67b9dd2d5456ca52fcd14d7d139",
          "url": "https://github.com/neohugo/neohugo/commit/f381a1160d073484bafc6fc4cc7d756ce4ec3e9b"
        },
        "date": 1606487771987,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 29.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "40875946 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 29.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "34049311 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4617,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "323455 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 82.2,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "12716436 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10095,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "114664 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 12392,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "94588 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 57398,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "21712 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 52977,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "22179 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2824,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "531646 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7148,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "168714 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 444597,
            "unit": "ns/op\t  103265 B/op\t    2220 allocs/op",
            "extra": "2512 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 25.4,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "44421685 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 360,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3390470 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 101923285,
            "unit": "ns/op\t35936481 B/op\t  799827 allocs/op",
            "extra": "14 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 179389,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "468183 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 685,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1708507 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1459,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "834513 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 941,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1274654 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 760,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1542603 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3439,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "366836 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 135,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8492961 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12200,
            "unit": "ns/op\t    6335 B/op\t     112 allocs/op",
            "extra": "99912 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1633,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "667600 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 282,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4437105 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2195,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "552849 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 143996,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "7484 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17118,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "70856 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 19975,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "57490 times\n2 procs"
          },
          {
            "name": "BenchmarkClassCollectorWriter",
            "value": 35526,
            "unit": "ns/op\t   34416 B/op\t     158 allocs/op",
            "extra": "33970 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 129076,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "9489 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6273,
            "unit": "ns/op\t    1989 B/op\t      46 allocs/op",
            "extra": "180176 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2543,
            "unit": "ns/op\t     503 B/op\t       2 allocs/op",
            "extra": "451026 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 214,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5527470 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 22607,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "52540 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 149779,
            "unit": "ns/op\t  161628 B/op\t    1204 allocs/op",
            "extra": "7042 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 245,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "5005078 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4147,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "281701 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1854,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "710572 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 67308,
            "unit": "ns/op\t   37730 B/op\t     179 allocs/op",
            "extra": "17641 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1141,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "928680 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 302,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4041476 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 598,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1971087 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.11,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "144969368 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 198,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6279670 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 212,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5715751 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 2983,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "381502 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1295,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "943036 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 442,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2730507 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1040,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 118,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "9862113 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 636,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1903926 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2312,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "560726 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 711,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1641352 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2530,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "488211 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 957,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1232018 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 326,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3514812 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1179,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "907579 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 345,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3773559 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1213,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 184243,
            "unit": "ns/op\t   50056 B/op\t     191 allocs/op",
            "extra": "6337 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2575,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "492771 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 229,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5408978 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1877,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "640910 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 254,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4670778 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 869,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1365236 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 455,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2577115 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 29440944,
            "unit": "ns/op\t 5459389 B/op\t   80032 allocs/op",
            "extra": "40 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 163,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7310038 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4539,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "257206 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4502,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "270764 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 116,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "11682058 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1855,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "644269 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 244,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4927700 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 7802,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "149025 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6116,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "206065 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 5816,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "212935 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3113,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "392034 times\n2 procs"
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
          "id": "5bf8695fcf1226699383c14ec5da7949e157ec72",
          "message": "Merge pull request #70 from blackb1rd/docs/disableHugoGeneratorInject_typo\n\ndocs: typo for disableHugoGeneratorInject",
          "timestamp": "2020-11-30T18:27:46+07:00",
          "tree_id": "90dcb63cfa7dec6003f3f4e4c3d69491a3dcd221",
          "url": "https://github.com/neohugo/neohugo/commit/5bf8695fcf1226699383c14ec5da7949e157ec72"
        },
        "date": 1606736359484,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 27.6,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "49147436 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 26.4,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "47334093 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4355,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "321688 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 80.6,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "15727653 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 10188,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "138848 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 11646,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "87165 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 51523,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "23638 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 48576,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "23347 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 4382,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "566596 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 6848,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "183451 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 421125,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2925 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 24.6,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "48205012 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 332,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3809260 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 99452910,
            "unit": "ns/op\t35944209 B/op\t  798913 allocs/op",
            "extra": "13 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 206062,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "461764 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 656,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1821321 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1361,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "872180 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 861,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1450635 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 723,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1681934 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3172,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "359896 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 132,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8843341 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 11703,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "101224 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1624,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "802375 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 260,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4315750 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2001,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "563738 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 128420,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "8979 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 16205,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "76058 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 18943,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "64542 times\n2 procs"
          },
          {
            "name": "BenchmarkClassCollectorWriter",
            "value": 32417,
            "unit": "ns/op\t   34427 B/op\t     158 allocs/op",
            "extra": "37204 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 122056,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "9978 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 5927,
            "unit": "ns/op\t    1980 B/op\t      46 allocs/op",
            "extra": "200960 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2406,
            "unit": "ns/op\t     505 B/op\t       3 allocs/op",
            "extra": "496638 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 206,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5732197 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 21737,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "54126 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 138745,
            "unit": "ns/op\t  161626 B/op\t    1204 allocs/op",
            "extra": "8682 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 234,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "5183072 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 3971,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "315898 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1742,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "720855 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 62414,
            "unit": "ns/op\t   37720 B/op\t     179 allocs/op",
            "extra": "18300 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1077,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 279,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4408594 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 571,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "2157532 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.36,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "157195767 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 177,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6374090 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 202,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "6218340 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 2894,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "430566 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1263,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "938074 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 420,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2855823 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 977,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1202469 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 111,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "11333866 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 596,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "2036766 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2131,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "611397 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 678,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1789501 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2535,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "501684 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 935,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1260258 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 316,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3699670 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1077,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 303,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3863976 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1109,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "986005 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 173065,
            "unit": "ns/op\t   50062 B/op\t     191 allocs/op",
            "extra": "7179 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2474,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "507897 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 222,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5711929 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1805,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "654243 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 248,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4901085 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 823,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1482372 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 426,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2786822 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 26544962,
            "unit": "ns/op\t 5459291 B/op\t   80032 allocs/op",
            "extra": "42 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 156,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7547803 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4228,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "283622 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4351,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "295473 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 132,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "10653043 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1768,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "693720 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 239,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "5040237 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 7525,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "162012 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 5786,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "209026 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 5739,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "218480 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3014,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "391962 times\n2 procs"
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
          "id": "e24f2b1f608c47d7689b30cc6a5ca4046da57cd2",
          "message": "Merge pull request #73 from blackb1rd/fix/bundle_resource_multilingual_custom_output\n\nfix: bundle resource is copied to wrong directory when Multilingual Mode and Custom output format",
          "timestamp": "2020-12-08T23:52:25+07:00",
          "tree_id": "1fa3e202caf4ef7210d533a28a563fba66208b19",
          "url": "https://github.com/neohugo/neohugo/commit/e24f2b1f608c47d7689b30cc6a5ca4046da57cd2"
        },
        "date": 1607447004958,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 28.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "43501077 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 27.6,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "40212976 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4580,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "308986 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 85.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "14396990 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 8785,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "114316 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 12488,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "96037 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 56830,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "20650 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 54177,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "21990 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 2482,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "486620 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 7558,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "164452 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 459070,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2498 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 27,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "44369211 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 363,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "3206352 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 108039446,
            "unit": "ns/op\t36712529 B/op\t  826752 allocs/op",
            "extra": "13 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 130879,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "438800 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 736,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "1642756 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1502,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "817766 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 951,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1244133 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 807,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1435444 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 3550,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "351092 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 143,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "8397654 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 12990,
            "unit": "ns/op\t    6336 B/op\t     112 allocs/op",
            "extra": "92848 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1723,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "643473 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 288,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4169768 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2275,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "542055 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 142904,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "7928 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 17926,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "67557 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 20011,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "61182 times\n2 procs"
          },
          {
            "name": "BenchmarkClassCollectorWriter",
            "value": 36506,
            "unit": "ns/op\t   34421 B/op\t     158 allocs/op",
            "extra": "31411 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 141278,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "9024 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 6734,
            "unit": "ns/op\t    1998 B/op\t      46 allocs/op",
            "extra": "165240 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2672,
            "unit": "ns/op\t     506 B/op\t       3 allocs/op",
            "extra": "458308 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 221,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5414769 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 24472,
            "unit": "ns/op\t    1345 B/op\t     120 allocs/op",
            "extra": "50125 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 163683,
            "unit": "ns/op\t  161618 B/op\t    1204 allocs/op",
            "extra": "6631 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 256,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "4747159 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 4337,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "279078 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1879,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "687810 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 73271,
            "unit": "ns/op\t   37733 B/op\t     179 allocs/op",
            "extra": "15906 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 1179,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "934365 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 300,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3996231 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 639,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "1926214 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 8.88,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "141723418 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 198,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "5842824 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 221,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "5591329 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 3201,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "359652 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1381,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "910638 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 462,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2643588 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 1069,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 123,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "9598254 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 650,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "1839769 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 2288,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "507530 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 741,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1607712 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2670,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "462586 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 1011,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 348,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3353236 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1238,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 324,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "3769407 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1242,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "870250 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 194008,
            "unit": "ns/op\t   50058 B/op\t     191 allocs/op",
            "extra": "6199 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2661,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "463213 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 236,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5086402 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1973,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "635310 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 270,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "4281508 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 891,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1369252 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 477,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2442081 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 29277518,
            "unit": "ns/op\t 5459512 B/op\t   80032 allocs/op",
            "extra": "39 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 172,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "7098716 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4667,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "255634 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4747,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "257690 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 118,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "11077915 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1959,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "580094 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 260,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "4637478 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 8258,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "150052 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 6601,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "194079 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 6219,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "198045 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 3376,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "374346 times\n2 procs"
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
          "id": "2821c62a06c6b13258e0da7335dfe7c61b5c3c57",
          "message": "Update README.md",
          "timestamp": "2020-12-15T11:02:13+07:00",
          "tree_id": "0eaee24a12f7af660081a911c4f1dd5c4dfdd1cd",
          "url": "https://github.com/neohugo/neohugo/commit/2821c62a06c6b13258e0da7335dfe7c61b5c3c57"
        },
        "date": 1608005530933,
        "tool": "go",
        "benches": [
          {
            "name": "BenchmarkIsTruthFul",
            "value": 24.8,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "51427528 times\n2 procs"
          },
          {
            "name": "BenchmarkScratchGet",
            "value": 23.7,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "48546825 times\n2 procs"
          },
          {
            "name": "BenchmarkStripHTML",
            "value": 4279,
            "unit": "ns/op\t     736 B/op\t       4 allocs/op",
            "extra": "393480 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentence",
            "value": 90.3,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "14395100 times\n2 procs"
          },
          {
            "name": "BenchmarkTestTruncateWordsToWholeSentenceOld",
            "value": 7679,
            "unit": "ns/op\t    2496 B/op\t       2 allocs/op",
            "extra": "140082 times\n2 procs"
          },
          {
            "name": "BenchmarkTotalWords",
            "value": 10432,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "115080 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiFprint",
            "value": 49531,
            "unit": "ns/op\t   31424 B/op\t      71 allocs/op",
            "extra": "24034 times\n2 procs"
          },
          {
            "name": "BenchmarkEmojiKyokomiSprint",
            "value": 47062,
            "unit": "ns/op\t   31344 B/op\t      66 allocs/op",
            "extra": "25269 times\n2 procs"
          },
          {
            "name": "BenchmarkHugoEmoji",
            "value": 3249,
            "unit": "ns/op\t     624 B/op\t      13 allocs/op",
            "extra": "577839 times\n2 procs"
          },
          {
            "name": "BenchmarkReaderContains",
            "value": 6297,
            "unit": "ns/op\t    1258 B/op\t      20 allocs/op",
            "extra": "186568 times\n2 procs"
          },
          {
            "name": "BenchmarkWalk",
            "value": 388859,
            "unit": "ns/op\t  103264 B/op\t    2220 allocs/op",
            "extra": "2595 times\n2 procs"
          },
          {
            "name": "BenchmarkGetGlob",
            "value": 22.5,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "52610760 times\n2 procs"
          },
          {
            "name": "BenchmarkGetPage",
            "value": 297,
            "unit": "ns/op\t      16 B/op\t       2 allocs/op",
            "extra": "4093696 times\n2 procs"
          },
          {
            "name": "BenchmarkResourceChainPostProcess",
            "value": 96704653,
            "unit": "ns/op\t35417620 B/op\t  780333 allocs/op",
            "extra": "14 times\n2 procs"
          },
          {
            "name": "BenchmarkReplaceShortcodeTokens",
            "value": 28449,
            "unit": "ns/op\t    3072 B/op\t       1 allocs/op",
            "extra": "414712 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorName",
            "value": 567,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "2189648 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameAsciiOnly",
            "value": 1162,
            "unit": "ns/op\t      48 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameBlackfriday",
            "value": 742,
            "unit": "ns/op\t     184 B/op\t       6 allocs/op",
            "extra": "1571068 times\n2 procs"
          },
          {
            "name": "BenchmarkSanitizeAnchorNameString",
            "value": 619,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "1950798 times\n2 procs"
          },
          {
            "name": "BenchmarkHowSimilar",
            "value": 2869,
            "unit": "ns/op\t     624 B/op\t      19 allocs/op",
            "extra": "408283 times\n2 procs"
          },
          {
            "name": "BenchmarkLayout",
            "value": 105,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "10072641 times\n2 procs"
          },
          {
            "name": "BenchmarkLayoutUncached",
            "value": 10735,
            "unit": "ns/op\t    6335 B/op\t     112 allocs/op",
            "extra": "114712 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyInterfaceMaps",
            "value": 1584,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "755737 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysStringsOnlyStringMaps",
            "value": 217,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "5650045 times\n2 procs"
          },
          {
            "name": "BenchmarkStringifyMapKeysIntegers",
            "value": 2105,
            "unit": "ns/op\t    1008 B/op\t       6 allocs/op",
            "extra": "508849 times\n2 procs"
          },
          {
            "name": "BenchmarkShortcodeLexer",
            "value": 111783,
            "unit": "ns/op\t  118572 B/op\t     916 allocs/op",
            "extra": "10780 times\n2 procs"
          },
          {
            "name": "BenchmarkParse",
            "value": 14101,
            "unit": "ns/op\t   16976 B/op\t      34 allocs/op",
            "extra": "84793 times\n2 procs"
          },
          {
            "name": "BenchmarkParseWithEmoji",
            "value": 16136,
            "unit": "ns/op\t   32960 B/op\t      16 allocs/op",
            "extra": "72226 times\n2 procs"
          },
          {
            "name": "BenchmarkClassCollectorWriter",
            "value": 29452,
            "unit": "ns/op\t   34417 B/op\t     158 allocs/op",
            "extra": "41282 times\n2 procs"
          },
          {
            "name": "BenchmarkRelatedMatchesIn",
            "value": 108356,
            "unit": "ns/op\t   20188 B/op\t     195 allocs/op",
            "extra": "11521 times\n2 procs"
          },
          {
            "name": "BenchmarkResizeParallel",
            "value": 5238,
            "unit": "ns/op\t    1977 B/op\t      46 allocs/op",
            "extra": "206220 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatch",
            "value": 2118,
            "unit": "ns/op\t     504 B/op\t       3 allocs/op",
            "extra": "577778 times\n2 procs"
          },
          {
            "name": "BenchmarkResourcesMatchA100",
            "value": 184,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "7393272 times\n2 procs"
          },
          {
            "name": "BenchmarkAssignMetadata",
            "value": 19200,
            "unit": "ns/op\t    1344 B/op\t     120 allocs/op",
            "extra": "62925 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeExif",
            "value": 132004,
            "unit": "ns/op\t  161634 B/op\t    1204 allocs/op",
            "extra": "9085 times\n2 procs"
          },
          {
            "name": "BenchmarkPageCache",
            "value": 210,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "5557304 times\n2 procs"
          },
          {
            "name": "BenchmarkSortByWeightAndReverse",
            "value": 3649,
            "unit": "ns/op\t      64 B/op\t       2 allocs/op",
            "extra": "337066 times\n2 procs"
          },
          {
            "name": "BenchmarkPermalinkExpand",
            "value": 1556,
            "unit": "ns/op\t     400 B/op\t      13 allocs/op",
            "extra": "810066 times\n2 procs"
          },
          {
            "name": "BenchmarkImportResolver",
            "value": 63272,
            "unit": "ns/op\t   37712 B/op\t     179 allocs/op",
            "extra": "18243 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaper",
            "value": 955,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1217383 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSEscaperNoSpecials",
            "value": 247,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4846639 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSS",
            "value": 523,
            "unit": "ns/op\t     160 B/op\t       1 allocs/op",
            "extra": "2268996 times\n2 procs"
          },
          {
            "name": "BenchmarkDecodeCSSNoSpecials",
            "value": 7.15,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "168511068 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilter",
            "value": 177,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "6808467 times\n2 procs"
          },
          {
            "name": "BenchmarkCSSValueFilterOk",
            "value": 184,
            "unit": "ns/op\t      48 B/op\t       3 allocs/op",
            "extra": "6253112 times\n2 procs"
          },
          {
            "name": "BenchmarkEscapedExecute",
            "value": 2682,
            "unit": "ns/op\t     624 B/op\t      18 allocs/op",
            "extra": "448771 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaper",
            "value": 1180,
            "unit": "ns/op\t     368 B/op\t       3 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkHTMLNospaceEscaperNoSpecials",
            "value": 378,
            "unit": "ns/op\t      32 B/op\t       1 allocs/op",
            "extra": "3141404 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTags",
            "value": 897,
            "unit": "ns/op\t     224 B/op\t       3 allocs/op",
            "extra": "1341379 times\n2 procs"
          },
          {
            "name": "BenchmarkStripTagsNoSpecials",
            "value": 101,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "12075350 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithNum",
            "value": 534,
            "unit": "ns/op\t      40 B/op\t       3 allocs/op",
            "extra": "2256868 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStr",
            "value": 1920,
            "unit": "ns/op\t     384 B/op\t       2 allocs/op",
            "extra": "634039 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithStrNoSpecials",
            "value": 620,
            "unit": "ns/op\t      96 B/op\t       2 allocs/op",
            "extra": "1928848 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObj",
            "value": 2237,
            "unit": "ns/op\t     448 B/op\t       3 allocs/op",
            "extra": "515709 times\n2 procs"
          },
          {
            "name": "BenchmarkJSValEscaperWithObjNoSpecials",
            "value": 847,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1412780 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaperNoSpecials",
            "value": 289,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4146369 times\n2 procs"
          },
          {
            "name": "BenchmarkJSStrEscaper",
            "value": 1015,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaperNoSpecials",
            "value": 270,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "4522816 times\n2 procs"
          },
          {
            "name": "BenchmarkJSRegexpEscaper",
            "value": 1048,
            "unit": "ns/op\t     336 B/op\t       2 allocs/op",
            "extra": "1000000 times\n2 procs"
          },
          {
            "name": "BenchmarkTemplateSpecialTags",
            "value": 169896,
            "unit": "ns/op\t   50067 B/op\t     191 allocs/op",
            "extra": "6834 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaper",
            "value": 2243,
            "unit": "ns/op\t     336 B/op\t       4 allocs/op",
            "extra": "546132 times\n2 procs"
          },
          {
            "name": "BenchmarkURLEscaperNoSpecials",
            "value": 193,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5837946 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizer",
            "value": 1538,
            "unit": "ns/op\t     176 B/op\t       3 allocs/op",
            "extra": "848878 times\n2 procs"
          },
          {
            "name": "BenchmarkURLNormalizerNoSpecials",
            "value": 241,
            "unit": "ns/op\t     112 B/op\t       2 allocs/op",
            "extra": "5084738 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilter",
            "value": 727,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "1562836 times\n2 procs"
          },
          {
            "name": "BenchmarkSrcsetFilterNoSpecials",
            "value": 424,
            "unit": "ns/op\t     160 B/op\t       3 allocs/op",
            "extra": "2960628 times\n2 procs"
          },
          {
            "name": "BenchmarkParseLarge",
            "value": 25426792,
            "unit": "ns/op\t 5459216 B/op\t   80032 allocs/op",
            "extra": "44 times\n2 procs"
          },
          {
            "name": "BenchmarkVariableString",
            "value": 144,
            "unit": "ns/op\t      72 B/op\t       3 allocs/op",
            "extra": "8247128 times\n2 procs"
          },
          {
            "name": "BenchmarkListString",
            "value": 4084,
            "unit": "ns/op\t    1608 B/op\t      31 allocs/op",
            "extra": "290406 times\n2 procs"
          },
          {
            "name": "BenchmarkPartial",
            "value": 4292,
            "unit": "ns/op\t    1152 B/op\t      37 allocs/op",
            "extra": "287571 times\n2 procs"
          },
          {
            "name": "BenchmarkPartialCached",
            "value": 124,
            "unit": "ns/op\t       0 B/op\t       0 allocs/op",
            "extra": "10808355 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalString",
            "value": 1648,
            "unit": "ns/op\t     720 B/op\t       4 allocs/op",
            "extra": "673678 times\n2 procs"
          },
          {
            "name": "BenchmarkUnmarshalResource",
            "value": 233,
            "unit": "ns/op\t     144 B/op\t       1 allocs/op",
            "extra": "5427046 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURL",
            "value": 7010,
            "unit": "ns/op\t    1568 B/op\t      16 allocs/op",
            "extra": "165786 times\n2 procs"
          },
          {
            "name": "BenchmarkAbsURLSrcset",
            "value": 5259,
            "unit": "ns/op\t    1288 B/op\t      23 allocs/op",
            "extra": "257940 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURLSrcset",
            "value": 5227,
            "unit": "ns/op\t    1366 B/op\t      21 allocs/op",
            "extra": "233418 times\n2 procs"
          },
          {
            "name": "BenchmarkXMLAbsURL",
            "value": 2816,
            "unit": "ns/op\t     928 B/op\t       8 allocs/op",
            "extra": "429927 times\n2 procs"
          }
        ]
      }
    ]
  }
}