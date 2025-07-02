## `yomichan-rs`  
_Rust library based off [Yomitan](https://github.com/yomidevs/yomitan)/[Yomichan_](https://github.com/FooSoft/yomichan)_
> [!CAUTION]
> **This library is highly unstable and will have breaking changes**

### Features/Roadmap
- [ ] **`Dictionary Imports`**
    - [x] Basic Dictionary Importing
        - [x] Index
        - [x] Tags
        - [x] Term
        - [x] TermMeta
        - [x] Kanji
        - [x] KanjiMeta
    - [ ] Advanced Importing
        - [ ] Dictionaries with Images/Media

### Multi-Language Deinflector [![github](https://img.shields.io/badge/github%20-blue.svg)](https://github.com/aramrw/deinflector) [![Crates.io](https://img.shields.io/crates/v/deinflector.svg)](https://crates.io/crates/deinflector) 
- [ ] [](https://github.com/aramrw/deinflector) 
- [ ] [**`Anki (Issue Tracker)`**](https://github.com/aramrw/yomichan_rs/issues/18)  
    - [ ] Note creation
        - [ ]  Basic Features
            - [ ] Notes 
                - [x] Create new from search
                - [ ] Edit Existing
                - [ ] Delete Existing
                - [ ] Overwrite Existing
        - [ ] Customization

- [ ] **`Misc`**
    - [ ] Definitions
        - [x] Plain Text (String)
        - [ ] Html

## Examples
```rust
let mut ycd = Yomichan::new("db.ycd");
ycd.set_language("ja");
let res = ycd.search("今勉強中です");
dbg!(res);
```
### Output example
* See the full type at [TermSearchResultSemgent](https://github.com/aramrw/yomichan_rs/blob/80156bee29e36c07f01018f7f33d1ee98fa965be/src/text_scanner.rs#L36)
```rust
[
    TermSearchResultsSegment {
        text: "今",
        results: Some(
            TermSearchResults {
                dictionary_entries: [
                    TermDictionaryEntry {
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        headwords: [
                            TermHeadword { term: "今", reading: "こん" },
                        ],
                        definitions: [
                            TermDefinition {
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "こん【今】\n\
                                                     〔教２〕\n\
                                                     コン・キン㊥\n\
                                                     いま\n\
                                                     筆順：\n\n\
                                                     （字義）\n\
                                                     ① いま。現在。このごろ。最近。「今代（きんだい）・今人（こんじん）・今日（こんにち）・昨今・自今・当今」\n\
                                                     ⇔昔(1)：昔(2)\n\
                                                     ② きょう。「今日（きよう）・今朝（けさ・こんちよう）・今夜」\n\
                                                     ③ こんど。このたび。「今回・今度・今年度」\n\
                                                     今宵（こよい）・今際（いまわ）",
                                    },
                                ],
                            },
                        ],
                    },
                    TermDictionaryEntry {
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        headwords: [
                            TermHeadword { term: "今", reading: "いま" },
                        ],
                        definitions: [
                            TermDefinition {
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "いま【今】\n\
                                                     🈩 （名）\n\
                                                     ① 過去と現在との間の一瞬間。この瞬間。現在の時点。「逃げるなら―だ」\n\
                                                     ② 現代。「―も通用している」「―小町」\n\
                                                     ⇔昔\n\
                                                     🈔 （副）\n\
                                                     ① すぐに。じきに。「―行きます」\n\
                                                     ② 今よりほんの少し前。「―着いたばかりだ」\n\
                                                     ③ 前にあったことに加えて、この時に。さらに。そのうえに。「―一度言ってごらん」\n\
                                                     現在・ただ今・現今・今日（こんにち）・今日日（きようび）・現時・現下・目下・当今・当世・当節・今頃（いまごろ）・今時分",
                                    },
                                ],
                            },
                        ],
                    },
                    // ... another entry for the reading "きん"
                ],
            },
        ),
    },
    TermSearchResultsSegment {
        text: "勉強",
        results: Some(
            TermSearchResults {
                dictionary_entries: [
                    TermDictionaryEntry {
                        // ../ dictionary,
                        // ../ headwords,
                        definitions: [
                            TermDefinition {
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "べん‐きょう【勉強】――キヤウ\n\
                                                     （名・自他スル）\n\
                                                     ① 学問や仕事など、物事に努め励むこと。\n\
                                                     ② 知識や技能を学ぶこと。「受験―」\n\
                                                     ③ 将来に役立つ経験や試練。「君にとっては、いい―だ」\n\
                                                     ④ 商品などを安く売ること。「端数は―します」",
                                    },
                                ],
                            },
                        ],
                    },
                ],
            },
        ),
    },
   ../ "中",
    ../ "です",
]
```

### Other Examples
1. [yomichan_rs_gui](http://github.com/aramrw/yomichan_rs_gui) (wip):
<img src="https://github.com/user-attachments/assets/b32cc484-8aa0-49e9-a68b-d2730ea173ea" width=350/>

