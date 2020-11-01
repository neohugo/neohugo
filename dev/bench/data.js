window.BENCHMARK_DATA = {
  "lastUpdate": 1604251727376,
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
      }
    ]
  }
}