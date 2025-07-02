## `yomichan-rs`  
Rust library based off [Yomitan](https://github.com/yomidevs/yomitan) 
> [!CAUTION]
> **This library is highly unstable and will have breaking changes**

### Features/Roadmap
- [ ] **Dictionary Imports**
    - [x] Basic Dictionary Importing
        - [x] Index
        - [x] Tags
        - [x] Term
        - [x] TermMeta
        - [x] Kanji
        - [x] KanjiMeta
    - [ ] Advanced Importing
        - [ ] Dictionaries with Images/Media

- [ ] [**Anki (Issue Tracker)**](https://github.com/aramrw/yomichan_rs/issues/18)     
    - [ ] Note creation
        - [ ]  Basic Features
            - [ ] Notes 
                - [x] Create new from search
                - [ ] Edit Existing
                - [ ] Delete Existing
                - [ ] Overwrite Existing
        - [ ] Customization

- [ ] **Deserialization**
    - [ ] Definitions
        - [x] Plain Text (String)
        - [ ] Html

### Examples
```rust
let mut ycd = Yomichan::new("db.ycd");
ycd.set_language("ja");
let res = ycd.search("今勉強中です");
dbg!(res);
```
_Example output, excluded some fields for brevity_
```rust
// Example output (abbreviated for clarity)
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
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        headwords: [
                            TermHeadword { term: "勉強", reading: "べんきょう" },
                        ],
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
    TermSearchResultsSegment {
        text: "中",
        results: Some(
            TermSearchResults {
                dictionary_entries: [
                    TermDictionaryEntry {
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        headwords: [
                            TermHeadword { term: "中", reading: "ちゅう" },
                        ],
                        definitions: [
                            TermDefinition {
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "ちゅう【中】\n\
                                                     〔教１〕\n\
                                                     チュウ・ジュウ\n\
                                                     なか・あたる・うち\n\
                                                     筆順：\n\n\
                                                     （字義）\n\
                                                     ① なか。うち。\n\
                                                     ㋐ まんなか。「中央・中心・正中」\n\
                                                     ㋑ 内部。「胸中・市中・車中・腹中」\n\
                                                     ㋒ ある時期の間。物事のまだ終わりきらないうち。「寒中・忌中・最中・道中」\n\
                                                     ㋓ 距離・時間などのなかほど。「中間・中秋・中旬・中途・中腹・中路」\n\
                                                     ㋔ なかま。「講中（こうじゆう）・連中」\n\
                                                     ㋕ 並み。ふつう。「中型・中流」\n\
                                                     ② ほどよい。かたよらない。「中正・中道・中庸・中立・中和」\n\

                                                     ③ あたる。\n\
                                                     ㋐ まとにあたる。「必中・命中・百発百中」\n\
                                                     ㋑ 予想と事実とが一致する。「的中・適中」\n\
                                                     ㋒ 体をそこなう。「中毒」\n\
                                                     ④ 「中国」の略。「米中仏」\n\
                                                     ⑤ 「中学校」の略。「中二」\n\
                                                     中心（なかご）・中稲（なかて）・中山道（なかせんどう）\n\
                                                     あつる・かなめ・すなお・ただ・ただし・な・なかば・のり・ひとし・まさ・みつ・みつる・よし",
                                    },
                                ],
                            },
                            // ... other definitions for "ちゅう"
                        ],
                    },
                    // ... other entries for "じゅう", "なか", etc.
                ],
            },
        ),
    },
    TermSearchResultsSegment {
        text: "です",
        results: Some(
            TermSearchResults {
                dictionary_entries: [
                    TermDictionaryEntry {
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        headwords: [
                            TermHeadword { term: "です", reading: "です" },
                        ],
                        definitions: [
                            TermDefinition {
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "です\n\
                                                     （助動－特殊型）《デシヨ・デシ・デス・（デス）・○・○》\n\
                                                     丁寧な断定の意を表す。「ここは学校―よ」「今晩は寒い―ね」\n\
                                                     形容詞の丁寧形は連用形に「ございます」を付けた「高うございます」の形が用いられたが、現在は「高いです」の形も用いられる。\n\
                                                     名詞および助詞（の・ほど・から・など・まで・だけ・くらい・ばかり、など）に付く。未然形「でしょ」に限り、動詞・形容詞・動詞型の助動詞・形容詞型の助動詞・特殊型の助動詞（ます・た・ぬ）の連体形に付く。連体形「です」は、ふつう助詞「ので」「のに」を伴い、「ですので」「ですのに」となる場合にだけ用いられる。\n\
                                                     【変遷】「でござります」→「でござんす」→「であんす」→「でんす」→「でえす」→「です」と変化したとする説、「で候（そうろう）」を略した「で候（そう）」の転とする説など諸説ある。動詞の終止形につく「～するです」などの形は、古い言い方や方言で用いられるが、現在の共通語では避けられ、「行くです」は「行きます」という。",
                                    },
                                ],
                            },
                        ],
                    },
                ],
            },
        ),
    },
]
```
