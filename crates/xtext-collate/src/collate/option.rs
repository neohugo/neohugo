//! Port of golang.org/x/text/collate/option.go.

use crate::colltab::collelem::{IDENTITY, NUM_LEVELS, QUATERNARY, SECONDARY, TERTIARY};
use crate::colltab::weighter::Weighter;
use crate::language::Tag;

/// alternateHandling identifies the various ways in which variables are
/// handled. A rune with a primary weight lower than the variable top is
/// considered a variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum AlternateHandling {
    /// altNonIgnorable turns off special handling of variables.
    NonIgnorable = 0,
    /// altBlanked sets variables and all subsequent primary ignorables to be
    /// ignorable at all levels.
    Blanked = 1,
    /// altShifted sets variables to be ignorable for levels one through three
    /// and adds a fourth level based on the values of the ignored levels.
    Shifted = 2,
    /// altShiftTrimmed is a slight variant of altShifted that is used to
    /// emulate POSIX.
    ShiftTrimmed = 3,
}

pub(crate) struct Options {
    /// ignore specifies which levels to ignore.
    pub ignore: [bool; NUM_LEVELS],
    /// caseLevel is true if there is an additional level of case matching
    /// between the secondary and tertiary levels.
    pub case_level: bool,
    /// backwards specifies the order of sorting at the secondary level.
    pub backwards: bool,
    /// numeric specifies whether any sequence of decimal digits (category is
    /// Nd) is sorted at a primary level with its numeric value.
    pub numeric: bool,
    /// alternate specifies an alternative handling of variables.
    pub alternate: AlternateHandling,
    /// variableTop is the largest primary value that is considered to be
    /// variable (set but never read, as in Go).
    #[allow(dead_code)]
    pub variable_top: u32,
    pub t: Box<dyn Weighter>,
    // f norm.Form (always norm.NFD) is not used by the collate code.
}

// Go: collate/option.go:newCollator (options part)
pub(crate) fn new_options(t: Box<dyn Weighter>) -> Options {
    let mut ignore = [false; NUM_LEVELS];
    ignore[QUATERNARY] = true;
    ignore[IDENTITY] = true;
    let vt = t.top();
    Options {
        ignore,
        case_level: false,
        backwards: false,
        numeric: false,
        alternate: AlternateHandling::NonIgnorable,
        // TODO: store vt in tags or remove.
        variable_top: vt,
        t,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum OptionKind {
    IgnoreCase,
    IgnoreDiacritics,
    IgnoreWidth,
    Loose,
    Force,
    Numeric,
    FromTag(Tag),
}

/// An Option is used to change the behavior of a Collator. Options override
/// the settings passed through the locale identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollOption {
    priority: i32,
    f: OptionKind,
}

/// IgnoreCase sets case-insensitive comparison.
pub const IGNORE_CASE: CollOption = CollOption {
    priority: 3,
    f: OptionKind::IgnoreCase,
};
/// IgnoreDiacritics causes diacritical marks to be ignored ("o" == "ö").
pub const IGNORE_DIACRITICS: CollOption = CollOption {
    priority: 3,
    f: OptionKind::IgnoreDiacritics,
};
/// IgnoreWidth causes full-width characters to match their half-width
/// equivalents.
pub const IGNORE_WIDTH: CollOption = CollOption {
    priority: 2,
    f: OptionKind::IgnoreWidth,
};
/// Loose sets the collator to ignore diacritics, case and width.
pub const LOOSE: CollOption = CollOption {
    priority: 4,
    f: OptionKind::Loose,
};
/// Force ordering if strings are equivalent but not equal.
pub const FORCE: CollOption = CollOption {
    priority: 5,
    f: OptionKind::Force,
};
/// Numeric specifies that numbers should sort numerically ("2" < "12").
pub const NUMERIC: CollOption = CollOption {
    priority: 5,
    f: OptionKind::Numeric,
};

// Go: collate/option.go:OptionsFromTag
/// Extracts the BCP47 collation options from the tag and configures a
/// collator accordingly. These options are set before any other option.
pub fn options_from_tag(t: &Tag) -> CollOption {
    CollOption {
        priority: 0,
        f: OptionKind::FromTag(t.clone()),
    }
}

impl Options {
    // Go: collate/option.go:options.setOptions
    pub(crate) fn set_options(&mut self, opts: &[CollOption]) {
        // Go: sort.Sort(prioritizedOptions(opts)) — pdqsort on the caller's
        // slice. Options of equal priority commute, so only the priority
        // order matters; go-sort reproduces Go's order anyway.
        let mut opts: Vec<CollOption> = opts.to_vec();
        go_sort::sort_by(&mut opts, |a, b| a.priority < b.priority);
        for x in &opts {
            match &x.f {
                OptionKind::IgnoreCase => ignore_case_f(self),
                OptionKind::IgnoreDiacritics => ignore_diacritics_f(self),
                OptionKind::IgnoreWidth => ignore_width_f(self),
                OptionKind::Loose => loose_f(self),
                OptionKind::Force => force_f(self),
                OptionKind::Numeric => numeric_f(self),
                OptionKind::FromTag(t) => self.set_from_tag(t),
            }
        }
    }

    // Go: collate/option.go:options.setFromTag
    pub(crate) fn set_from_tag(&mut self, t: &Tag) {
        self.case_level = ldml_bool(t, self.case_level, "kc");
        self.backwards = ldml_bool(t, self.backwards, "kb");
        self.numeric = ldml_bool(t, self.numeric, "kn");

        // Extract settings from the BCP47 u extension.
        match t.type_for_key("ks").as_str() {
            // strength
            "level1" => {
                self.ignore[SECONDARY] = true;
                self.ignore[TERTIARY] = true;
            }
            "level2" => {
                self.ignore[TERTIARY] = true;
            }
            "level3" | "" => {
                // The default.
            }
            "level4" => {
                self.ignore[QUATERNARY] = false;
            }
            "identic" => {
                self.ignore[QUATERNARY] = false;
                self.ignore[IDENTITY] = false;
            }
            _ => {}
        }

        match t.type_for_key("ka").as_str() {
            "shifted" => self.alternate = AlternateHandling::Shifted,
            // The following two types are not official BCP47, but we support
            // them to give access to this otherwise hidden functionality.
            "blanked" => self.alternate = AlternateHandling::Blanked,
            "posix" => self.alternate = AlternateHandling::ShiftTrimmed,
            _ => {}
        }

        // TODO: caseFirst ("kf"), reorder ("kr"), and maybe variableTop ("vt").
    }
}

// Go: collate/option.go:ldmlBool
fn ldml_bool(t: &Tag, old: bool, key: &str) -> bool {
    match t.type_for_key(key).as_str() {
        "true" => true,
        "false" => false,
        _ => old,
    }
}

// Go: collate/option.go:ignoreWidthF
fn ignore_width_f(o: &mut Options) {
    o.ignore[TERTIARY] = true;
    o.case_level = true;
}

// Go: collate/option.go:ignoreDiacriticsF
fn ignore_diacritics_f(o: &mut Options) {
    o.ignore[SECONDARY] = true;
}

// Go: collate/option.go:ignoreCaseF
fn ignore_case_f(o: &mut Options) {
    o.ignore[TERTIARY] = true;
    o.case_level = false;
}

// Go: collate/option.go:looseF
fn loose_f(o: &mut Options) {
    ignore_width_f(o);
    ignore_diacritics_f(o);
    ignore_case_f(o);
}

// Go: collate/option.go:forceF
fn force_f(o: &mut Options) {
    o.ignore[IDENTITY] = false;
}

// Go: collate/option.go:numericF
fn numeric_f(o: &mut Options) {
    o.numeric = true;
}
