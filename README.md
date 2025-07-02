## `yomichan-rs`  

### Examples
```rust
let mut ycd = Yomichan::new("db.ycd");
ycd.set_language("ja");
let res = ycd.search("今勉強中です");
dbg!(res);
```
// Output of `dbg!(res);` (simplified for brevity)
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
                            TermHeadword {
                                term: "今",
                                reading: "こん",
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "こん【今】\n〔教２〕\nコン・キン㊥\nいま\n筆順：\n\n（字義）\n① いま。現在。このごろ。最近。「今代（きんだい）・今人（こんじん）・今日（こんにち）・昨今・自今・当今」...",
                                    },
                                ],
                            },
                        ],
                    },
                    TermDictionaryEntry {
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        headwords: [
                            TermHeadword {
                                term: "今",
                                reading: "いま",
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "いま【今】\n🈩 （名）\n① 過去と現在との間の一瞬間。この瞬間。現在の時点。「逃げるなら―だ」...",
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
                            TermHeadword {
                                term: "勉強",
                                reading: "べんきょう",
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "べん‐きょう【勉強】――キヤウ\n（名・自他スル）\n① 学問や仕事など、物事に努め励むこと。\n② 知識や技能を学ぶこと。「受験―」...",
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
                            TermHeadword {
                                term: "中",
                                reading: "ちゅう",
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "ちゅう【中】\n〔教１〕\nチュウ・ジュウ\nなか・あたる ・うち\n筆順：\n\n（字義）\n① なか。うち。\n㋐ まんなか。「中央・中心・正中」...",
                                    },
                                ],
                            },
                            TermDefinition {
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "‐ちゅう【中】\n（接尾）\n① …の間。うち。「来月―」「十一―八九」\n② …をしている間。最中。「授業―」\n③ なか。「海水―」",
                                    },
                                ],
                            },
                            // ... another definition for "ちゅう"
                        ],
                    },
                    TermDictionaryEntry {
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        headwords: [
                            TermHeadword {
                                term: "中",
                                reading: "じゅう",
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "‐じゅう【中】ヂユウ\n（接尾）\n① 期間を表す語に付いて、その間ずっと続く意を添える。…の間。「一年―雨が多い」...",
                                    },
                                ],
                            },
                            // ... other definitions for "じゅう" and other entries for "なか", etc.
                        ],
                    },
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
                            TermHeadword {
                                term: "です",
                                reading: "です",
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "です\n（助動－特殊型）《デシヨ・デシ・デス・（デス） ・○・○》\n丁寧な断定の意を表す。「ここは学校―よ」「今晩は寒い―ね」...",
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
