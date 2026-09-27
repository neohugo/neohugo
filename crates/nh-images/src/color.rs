//! Port of `resources/images/color.go`.
//!
//! Owner: Wave B task T10 (images).


// Wave B: port or stub per the checklist below (not exercised by seeksnack unless noted).

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/color.go (202 lines; 1/15 funcs executed)
//   types: colorGoProvider, Color
//    L45-47: (c Color) Luminance() float64
//    L51-53: (c Color) ColorGo() color.Color
//    L56-58: (c Color) ColorHex() string
//    L61-63: (c Color) String() string
//    L68-72: (c Color) Hash() (uint64, error)
//    L74-79: (c *Color) init() error
//    L81-88: (c Color) toSRGB(i uint8) float64
//    L93-104: AddColorToPalette(c color.Color, p color.Palette) color.Palette
//    L108-110: ReplaceColorInPalette(c color.Color, p color.Palette)
//    L113-120: ColorGoToHexString(c color.Color) string
//    L123-129: ColorGoToColor(c color.Color) Color
//    L131-137: hexStringToColor(s string) Color
//    L140-146: HexStringsToColors(s ...string) []Color
//    L148-163: toColorGo(v any) (color.Color, bool, error)
// EX L165-202: hexStringToColorGo(s string) (color.Color, error)
// ---------------------------------------------------------------------------
