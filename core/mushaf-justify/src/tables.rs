//! Letter sets and patterns of the oracle (quran-madina-html `justify.py` @ d520c5c),
//! generated from its source and constants, not typed by hand. Do not edit.

/// A set of characters.
pub(crate) struct Set(pub &'static str);

impl Set {
    pub(crate) fn contains(&self, c: char) -> bool {
        self.0.contains(c)
    }
}

pub(crate) const AIN_GHAIN: Set = Set("\u{0639}\u{063A}");
pub(crate) const ALEF_DAL_LAM: Set =
    Set("\u{0622}\u{0627}\u{062F}\u{0630}\u{0671}\u{0623}\u{0625}\u{0644}");
pub(crate) const ALT_BEH_KAF_NOON: Set = Set("\u{0628}\u{062A}\u{062B}\u{0643}\u{0646}");
pub(crate) const ALT_YEH_SAD_SEEN_FEH: Set =
    Set("\u{0649}\u{0635}\u{0636}\u{0633}\u{0634}\u{0641}\u{0642}\u{064A}\u{0626}");
pub(crate) const BEH_LIKE: Set = Set("\u{0628}\u{062A}\u{062B}\u{0646}\u{064A}\u{0626}");
pub(crate) const FEH_QAF: Set = Set("\u{0641}\u{0642}");
pub(crate) const FINAL_ASCENDANT: Set =
    Set("\u{0622}\u{0627}\u{062F}\u{0630}\u{0671}\u{0623}\u{0625}\u{0643}\u{0644}\u{0647}\u{0629}");
pub(crate) const HAH_LIKE: Set = Set("\u{062C}\u{062D}\u{062E}");
pub(crate) const HEH: Set = Set("\u{0647}");
pub(crate) const HEH_TEH_MARBUTA: Set = Set("\u{0647}\u{0629}");
pub(crate) const KAF_DAL_THAL_TEH_MARBUTA: Set = Set("\u{0643}\u{062F}\u{0630}\u{0629}");
pub(crate) const L: Set = Set(
    "\u{0626}\u{0628}\u{062A}\u{062B}\u{0646}\u{064A}\u{062C}\u{062D}\u{062E}\u{0637}\u{0638}\u{0639}\u{063A}\u{0641}\u{0642}\u{0629}\u{0644}\u{0645}",
);
pub(crate) const LAM: Set = Set("\u{0644}");
pub(crate) const MEEM: Set = Set("\u{0645}");
pub(crate) const QAF: Set = Set("\u{0642}");
pub(crate) const R: Set = Set(
    "\u{0628}\u{062A}\u{062B}\u{0646}\u{064A}\u{0626}\u{062C}\u{062D}\u{062E}\u{0633}\u{0634}\u{0635}\u{0636}\u{0637}\u{0638}\u{0639}\u{063A}\u{0641}\u{0642}\u{0645}\u{0647}",
);
pub(crate) const REH_ZAIN: Set = Set("\u{0631}\u{0632}");
pub(crate) const REH_ZAIN_NOON: Set = Set("\u{0631}\u{0632}\u{0646}");
pub(crate) const RIGHT_NO_JOIN: Set = Set(
    "\u{0622}\u{0627}\u{0671}\u{0623}\u{0625}\u{062F}\u{0630}\u{0631}\u{0632}\u{0648}\u{0624}\u{0621}\u{0629}",
);
pub(crate) const SEEN_SAD: Set = Set("\u{0633}\u{0634}\u{0635}\u{0636}");
pub(crate) const SEEN_SHEEN: Set = Set("\u{0633}\u{0634}");
pub(crate) const YEH_BEH_TEH_THEH_NOON_YEH_MAKSURA: Set =
    Set("\u{0626}\u{0628}\u{062A}\u{062B}\u{0646}\u{064A}\u{0649}");
pub(crate) const BASES: Set = Set(
    "\u{0628}\u{062A}\u{062B}\u{062C}\u{062D}\u{062E}\u{0633}\u{0634}\u{0635}\u{0636}\u{0637}\u{0638}\u{0639}\u{063A}\u{0641}\u{0642}\u{0643}\u{0644}\u{0645}\u{0646}\u{0647}\u{064A}\u{0626}\u{0649}\u{0622}\u{0627}\u{0671}\u{0623}\u{0625}\u{062F}\u{0630}\u{0631}\u{0632}\u{0648}\u{0624}\u{0621}\u{0629}",
);
pub(crate) const FATHA: char = '\u{064E}';
pub(crate) const SHADDA: char = '\u{0651}';
pub(crate) const HAMZA: char = '\u{0621}';
pub(crate) const AYAH_END: char = '\u{06DD}';

pub(crate) const RX_BEH: &str = "^.+([\u{0628}\u{062A}\u{062B}\u{0646}\u{064A}\u{0633}\u{0634}\u{0635}\u{0636}][\u{0628}\u{062A}\u{062B}\u{0646}\u{064A}\u{0645}]).+$";
pub(crate) const RX_KAF: &str = "^.*([\u{0643}].).*$";

pub(crate) fn rx_fina() -> String {
    format!("^.*([{}][{}])$", R.0, FINAL_ASCENDANT.0)
}
pub(crate) fn rx_fina_unanchored() -> String {
    format!("([{}][{}])$", R.0, FINAL_ASCENDANT.0)
}
pub(crate) fn rx_reh() -> String {
    format!(".*([{}][\u{0631}\u{0632}])", R.0)
}
pub(crate) fn rx_reh_bare() -> String {
    format!("([{}][\u{0631}\u{0632}])", R.0)
}
pub(crate) fn rx_other() -> String {
    format!(".*([{}](?:[\u{0644}]|[{}]))", R.0, L.0)
}
pub(crate) fn rx_other_bare() -> String {
    format!("([{}](?:[\u{0644}]|[{}]))", R.0, L.0)
}
