# Glossary

Arabic terms, their transliteration, and the identifier used in code and data. Identifiers are
lowercase ASCII with underscores. The glosses are short working definitions for developers;
the owner corrects them. A category's exact membership and wujūh always come from the books
(rule 1), never from this page.

## Transmission

| Term | Transliteration | Code | Meaning here |
| --- | --- | --- | --- |
| قراءة / قراءات | qirāʾah / qirāʾāt | `qiraah` | the reading of one of the ten qurrāʾ |
| قارئ / قرّاء | qāriʾ / qurrāʾ | `qari` | one of the ten imams (`nafi`, `ibn_kathir`, …) |
| رواية | riwāyah | `riwayah` | a rāwī's transmission of a qāriʾ (`warsh`, `qalun`, …); the unit the app displays |
| راوٍ / رواة | rāwī / ruwāt | `rawi` | one of the twenty transmitters |
| طريق / طرق | ṭarīq / ṭuruq | `tariq` | a line of transmission below the rāwī; in scope: al-Shāṭibiyyah and al-Durrah |
| وجه / أوجه | wajh / wujūh (awjuh) | `wajh` | one permitted way of reading a place; a riwāyah may have several |
| تحرير / تحريرات | taḥrīr / taḥrīrāt | `allowed_combinations` | rules on which wujūh may be combined |
| مقرئ | muqriʾ | `muqri` | a teacher with ijāzah who verifies readings |

## What a reading changes

| Term | Transliteration | Code | Meaning here |
| --- | --- | --- | --- |
| فرش الحروف | farsh al-ḥurūf | `farsh` | word-specific differences, listed place by place |
| أصول | uṣūl | `usul` | systematic features that apply wherever their conditions hold |
| رسم | rasm | `rasm` | the consonantal skeleton as written in the maṣāḥif |
| ضبط | ḍabṭ | `dabt` | the marking of vowels and signs; differs between printing traditions |
| عدّ الآي | ʿadd al-āy | `adad` | an ayah counting system (Kūfī, Madanī awwal and ākhir, Makkī, Baṣrī, Shāmī, Ḥimṣī) |
| وقف / وصل | waqf / waṣl | `waqf`, `wasl` | pausing / continuing; some wujūh exist only in one |

## Uṣūl categories (names only)

| Term | Transliteration | Code |
| --- | --- | --- |
| المد المتصل | al-madd al-muttaṣil | `muttasil` |
| المد المنفصل | al-madd al-munfaṣil | `munfasil` |
| مد البدل | madd al-badal | `badal` |
| مد اللين | madd al-līn | `lin` |
| المد العارض للسكون | al-madd al-ʿāriḍ li-l-sukūn | `arid` |
| المد اللازم | al-madd al-lāzim | `lazim` |
| ميم الجمع | mīm al-jamʿ | `mim_al_jam` |
| هاء الكناية | hāʾ al-kināyah | `ha_al_kinayah` |
| الهمز المفرد / الهمزتان | al-hamz al-mufrad / al-hamzatān | `hamz_mufrad`, `hamzatan` |
| النقل | al-naql | `naql` |
| السكت | al-sakt | `sakt` |
| الإبدال | al-ibdāl | `ibdal` |
| التسهيل | al-tas-hīl | `tashil` |
| الإدغام الكبير / الصغير | al-idghām al-kabīr / al-ṣaghīr | `idgham_kabir`, `idgham_saghir` |
| الفتح والإمالة والتقليل | al-fatḥ, al-imālah, al-taqlīl | `fath`, `imalah`, `taqlil` |
| ذوات الياء | dhawāt al-yāʾ | `dhawat_al_ya` |
| الراءات | al-rāʾāt (tarqīq, tafkhīm) | `raat` |
| اللامات | al-lāmāt (taghlīẓ) | `lamat` |
| ياءات الإضافة / الزوائد | yāʾāt al-iḍāfah / al-zawāʾid | `ya_idafah`, `ya_zawaid` |
| الروم والإشمام | al-rawm, al-ishmām | `rawm`, `ishmam` |

Madd lengths as wujūh: `qasr`, `tawassut`, `tul` (also called `madd` / `ishba` in the books; the
profile records the book's own term in its row).

## Sign features (D72 level b, M1.2)

What a mark means in a writing style; rows in `corpus/dabt/<style>.yaml`, each with a citation.
Names from the uṣūl table above are reused (`sakt`, `tashil`, `imalah`, `ishmam`).

| Feature | Meaning (KFGQPC Madinah explanation pages) |
| --- | --- |
| `not_pronounced` | a letter added in the rasm, not pronounced in waṣl or waqf (round zero) |
| `dropped_in_wasl` | an alif dropped in waṣl and pronounced in waqf (oblong zero) |
| `vowelless`, `izhar` | the sukūn sign: the letter is vowelless and pronounced clearly |
| `tanwin_fath`, `tanwin_damm`, `tanwin_kasr` | which tanwīn |
| `tanwin_izhar`, `tanwin_not_izhar` | stacked tanwīn (iẓhār) or consecutive (idghām or ikhfāʾ, by the next letter) |
| `iqlab` | the small mīm: iqlāb before bāʾ |
| `small_letter_alef`, `small_letter_yeh`, `small_letter_noon` | a small letter: left out of the rasm, pronounced |
| `madd_beyond_natural` | the madd sign: longer than the natural madd |
| `sin_for_sad`, `sad_preferred` | sīn over a ṣād (read with sīn) / under it (ṣād the better known) |
| `sajdah_word`, `sajdah_place`, `hizb_start` | the line over a sajdah word, the sajdah sign, the ḥizb sign |
| `waqf_lazim`, `waqf_jaiz`, `wasl_awla`, `waqf_awla`, `waqf_muanaqah` | the stop signs |
| `fatha`, `damma`, `kasra`, `shadda`, `hamza_above`, `hamza_below` | `hypothesis`: not defined on the pages |

## Sources

| Book | Code | Role (D68) |
| --- | --- | --- |
| البدور الزاهرة، عبد الفتاح القاضي | `al-budur` | source of truth |
| الفتوح الباهرة | `al-futuh` | source of truth |
| مصاحف مجمع الملك فهد | `kfgqpc-<riwayah>` | check only |
