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

## Sources

| Book | Code | Role (D68) |
| --- | --- | --- |
| البدور الزاهرة، عبد الفتاح القاضي | `al-budur` | source of truth |
| الفتوح الباهرة | `al-futuh` | source of truth |
| مصاحف مجمع الملك فهد | `kfgqpc-<riwayah>` | check only |
