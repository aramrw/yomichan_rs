## `yomichan-rs`  

### Examples
```rust
let mut ycd = Yomichan::new("db.ycd");
ycd.set_language("ja");
let res = ycd.search("今勉強中です");
dbg!(res);
```
output:
```rust
[src\text_scanner.rs:526:9] res = [
    TermSearchResultsSegment {
        text: "今",
        results: Some(
            TermSearchResults {
                dictionary_entries: [
                    TermDictionaryEntry {
                        entry_type: Term,
                        is_primary: true,
                        text_processor_rule_chain_candidates: [
                            [],
                        ],
                        inflection_rule_chain_candidates: [
                            InflectionRuleChainCandidate {
                                source: Algorithm,
                                inflection_rules: [],
                            },
                        ],
                        score: 1,
                        frequency_order: 0,
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        dictionary_index: 8,
                        source_term_exact_match_count: 1,
                        match_primary_reading: false,
                        max_original_text_length: 3,
                        headwords: [
                            TermHeadword {
                                index: 0,
                                term: "今",
                                reading: "こん",
                                sources: [
                                    TermSource {
                                        original_text: "今",
                                        transformed_text: "今",
                                        deinflected_text: "今",
                                        match_type: Exact,
                                        match_source: Term,
                                        is_primary: true,
                                    },
                                ],
                                tags: [],
                                word_classes: [],
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                id: "0197c995-e263-75d1-8f36-b3fd68ae8ee6",
                                index: 0,
                                headword_indices: [
                                    0,
                                ],
                                dictionary: "旺文社国語辞典 第十一版 画像無し",
                                dictionary_index: 8,
                                dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                                score: 1,
                                frequency_order: 0,
                                sequences: [
                                    52756,
                                ],
                                is_primary: true,
                                tags: [],
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "こん【今】\n〔教２〕\nコン・キン㊥\nいま\n筆順：\n\n 
（字義）\n① いま。現在。このごろ。最近。「今代（きんだい）・今人（こんじん）・今日（こんにち）・昨今・自今
・当今」\n⇔昔(1)：昔(2)\n② きょう。「今日（きよう）・今朝（けさ・こんちよう）・今夜」\n③ こんど。このたび 
。「今回・今度・今年度」\n今宵（こよい）・今際（いまわ）",
                                        html: None,
                                    },
                                ],
                            },
                        ],
                        pronunciations: [],
                        frequencies: [],
                    },
                    TermDictionaryEntry {
                        entry_type: Term,
                        is_primary: true,
                        text_processor_rule_chain_candidates: [
                            [],
                        ],
                        inflection_rule_chain_candidates: [
                            InflectionRuleChainCandidate {
                                source: Algorithm,
                                inflection_rules: [],
                            },
                        ],
                        score: 1,
                        frequency_order: 0,
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        dictionary_index: 8,
                        source_term_exact_match_count: 1,
                        match_primary_reading: false,
                        max_original_text_length: 3,
                        headwords: [
                            TermHeadword {
                                index: 0,
                                term: "今",
                                reading: "いま",
                                sources: [
                                    TermSource {
                                        original_text: "今",
                                        transformed_text: "今",
                                        deinflected_text: "今",
                                        match_type: Exact,
                                        match_source: Term,
                                        is_primary: true,
                                    },
                                ],
                                tags: [],
                                word_classes: [],
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                id: "0197c995-e26f-7d33-8ba1-c3536cb3e6a6",
                                index: 0,
                                headword_indices: [
                                    0,
                                ],
                                dictionary: "旺文社国語辞典 第十一版 画像無し",
                                dictionary_index: 8,
                                dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                                score: 1,
                                frequency_order: 0,
                                sequences: [
                                    8740,
                                ],
                                is_primary: true,
                                tags: [],
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "いま【今】\n🈩 （名）\n① 過去と現在との間の一瞬間。こ
の瞬間。現在の時点。「逃げるなら―だ」\n② 現代。「―も通用している」「―小町」\n⇔昔\n🈔 （副）\n① すぐに。じ 
きに。「―行きます」\n② 今よりほんの少し前。「―着いたばかりだ」\n③ 前にあったことに加えて、この時に。さらに
。そのうえに。「―一度言ってごらん」\n現在・ただ今・現今・今日（こんにち）・今日日（きようび）・現時・現下 
・目下・当今・当世・当節・今頃（いまごろ）・今時分",
                                        html: None,
                                    },
                                ],
                            },
                        ],
                        pronunciations: [],
                        frequencies: [],
                    },
                    TermDictionaryEntry {
                        entry_type: Term,
                        is_primary: true,
                        text_processor_rule_chain_candidates: [
                            [],
                        ],
                        inflection_rule_chain_candidates: [
                            InflectionRuleChainCandidate {
                                source: Algorithm,
                                inflection_rules: [],
                            },
                        ],
                        score: 0,
                        frequency_order: 0,
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        dictionary_index: 8,
                        source_term_exact_match_count: 1,
                        match_primary_reading: false,
                        max_original_text_length: 3,
                        headwords: [
                            TermHeadword {
                                index: 0,
                                term: "今",
                                reading: "きん",
                                sources: [
                                    TermSource {
                                        original_text: "今",
                                        transformed_text: "今",
                                        deinflected_text: "今",
                                        match_type: Exact,
                                        match_source: Term,
                                        is_primary: true,
                                    },
                                ],
                                tags: [],
                                word_classes: [],
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                id: "0197c995-e25e-7db1-b740-8edeb97b311a",
                                index: 0,
                                headword_indices: [
                                    0,
                                ],
                                dictionary: "旺文社国語辞典 第十一版 画像無し",
                                dictionary_index: 8,
                                dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                                score: 0,
                                frequency_order: 0,
                                sequences: [
                                    36566,
                                ],
                                is_primary: true,
                                tags: [],
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "きん【今】\n（字義）→こん（今）",
                                        html: None,
                                    },
                                ],
                            },
                        ],
                        pronunciations: [],
                        frequencies: [],
                    },
                ],
                sentence: Sentence {
                    text: "今勉強中です",
                    offset: 0,
                },
            },
        ),
    },
    TermSearchResultsSegment {
        text: "勉強",
        results: Some(
            TermSearchResults {
                dictionary_entries: [
                    TermDictionaryEntry {
                        entry_type: Term,
                        is_primary: true,
                        text_processor_rule_chain_candidates: [
                            [],
                        ],
                        inflection_rule_chain_candidates: [
                            InflectionRuleChainCandidate {
                                source: Algorithm,
                                inflection_rules: [],
                            },
                        ],
                        score: 1,
                        frequency_order: 0,
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        dictionary_index: 6,
                        source_term_exact_match_count: 1,
                        match_primary_reading: false,
                        max_original_text_length: 6,
                        headwords: [
                            TermHeadword {
                                index: 0,
                                term: "勉強",
                                reading: "べんきょう",
                                sources: [
                                    TermSource {
                                        original_text: "勉強",
                                        transformed_text: "勉強",
                                        deinflected_text: "勉強",
                                        match_type: Exact,
                                        match_source: Term,
                                        is_primary: true,
                                    },
                                ],
                                tags: [],
                                word_classes: [],
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                id: "0197c995-e235-7221-b4e9-1f637d13128f",
                                index: 0,
                                headword_indices: [
                                    0,
                                ],
                                dictionary: "旺文社国語辞典 第十一版 画像無し",
                                dictionary_index: 6,
                                dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                                score: 1,
                                frequency_order: 0,
                                sequences: [
                                    134448,
                                ],
                                is_primary: true,
                                tags: [],
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "べん‐きょう【勉強】――キヤウ\n（名・自他スル）\n① 学問
や仕事など、物事に努め励むこと。\n② 知識や技能を学ぶこと。「受験―」\n③ 将来に役立つ経験や試練。「君にとっ 
ては、いい―だ」\n④ 商品などを安く売ること。「端数は―します」",
                                        html: None,
                                    },
                                ],
                            },
                        ],
                        pronunciations: [],
                        frequencies: [],
                    },
                ],
                sentence: Sentence {
                    text: "今勉強中です",
                    offset: 0,
                },
            },
        ),
    },
    TermSearchResultsSegment {
        text: "中",
        results: Some(
            TermSearchResults {
                dictionary_entries: [
                    TermDictionaryEntry {
                        entry_type: Term,
                        is_primary: true,
                        text_processor_rule_chain_candidates: [
                            [],
                        ],
                        inflection_rule_chain_candidates: [
                            InflectionRuleChainCandidate {
                                source: Algorithm,
                                inflection_rules: [],
                            },
                        ],
                        score: 1,
                        frequency_order: 0,
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        dictionary_index: 5,
                        source_term_exact_match_count: 1,
                        match_primary_reading: false,
                        max_original_text_length: 3,
                        headwords: [
                            TermHeadword {
                                index: 0,
                                term: "中",
                                reading: "ちゅう",
                                sources: [
                                    TermSource {
                                        original_text: "中",
                                        transformed_text: "中",
                                        deinflected_text: "中",
                                        match_type: Exact,
                                        match_source: Term,
                                        is_primary: true,
                                    },
                                ],
                                tags: [],
                                word_classes: [],
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                id: "0197c995-e2a9-79c0-b569-1c31603690f0",
                                index: 0,
                                headword_indices: [
                                    0,
                                ],
                                dictionary: "旺文社国語辞典 第十一版 画像無し",
                                dictionary_index: 5,
                                dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                                score: 1,
                                frequency_order: 0,
                                sequences: [
                                    94656,
                                ],
                                is_primary: true,
                                tags: [],
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "ちゅう【中】\n〔教１〕\nチュウ・ジュウ\nなか・あたる 
・うち\n筆順：\n\n（字義）\n① なか。うち。\n㋐ まんなか。「中央・中心・正中」\n㋑ 内部。「胸中・市中・車中
・腹中」\n㋒ ある時期の間。物事のまだ終わりきらないうち。「寒中・忌中・最中・道中」\n㋓ 距離・時間などのな
かほど。「中間・中秋・中旬・中途・中腹・中路」\n㋔ なかま。「講中（こうじゆう）・連中」\n㋕ 並み。ふつう。
「中型・中流」\n② ほどよい。かたよらない。「中正・中道・中庸・中立・中和」\n③ あたる。\n㋐ まとにあたる。 
「必中・命中・百発百中」\n㋑ 予想と事実とが一致する。「的中・適中」\n㋒ 体をそこなう。「中毒」\n④ 「中国」
の略。「米中仏」\n⑤ 「中学校」の略。「中二」\n中心（なかご）・中稲（なかて）・中山道（なかせんどう）\nあつ
る・かなめ・すなお・ただ・ただし・な・なかば・のり・ひとし・まさ・みつ・みつる・よし",
                                        html: None,
                                    },
                                ],
                            },
                            TermDefinition {
                                id: "0197c995-e2a9-79c0-b569-1c40693db5b6",
                                index: 1,
                                headword_indices: [
                                    0,
                                ],
                                dictionary: "旺文社国語辞典 第十一版 画像無し",
                                dictionary_index: 5,
                                dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                                score: 1,
                                frequency_order: 0,
                                sequences: [
                                    94658,
                                ],
                                is_primary: true,
                                tags: [],
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "‐ちゅう【中】\n（接尾）\n① …の間。うち。「来月―」「十
―八九」\n② …をしている間。最中。「授業―」\n③ なか。「海水―」",
                                        html: None,
                                    },
                                ],
                            },
                            TermDefinition {
                                id: "0197c995-e2a9-79c0-b569-1c52ff391dbb",
                                index: 2,
                                headword_indices: [
                                    0,
                                ],
                                dictionary: "旺文社国語辞典 第十一版 画像無し",
                                dictionary_index: 5,
                                dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                                score: 1,
                                frequency_order: 0,
                                sequences: [
                                    94660,
                                ],
                                is_primary: true,
                                tags: [],
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "ちゅう【中】\n① なかほど。並み。ふつう。「―ぐらい」「
―の品」「上―下」\n② かたよらないこと。中庸。「―を取る」",
                                        html: None,
                                    },
                                ],
                            },
                        ],
                        pronunciations: [],
                        frequencies: [],
                    },
                    TermDictionaryEntry {
                        entry_type: Term,
                        is_primary: true,
                        text_processor_rule_chain_candidates: [
                            [],
                        ],
                        inflection_rule_chain_candidates: [
                            InflectionRuleChainCandidate {
                                source: Algorithm,
                                inflection_rules: [],
                            },
                        ],
                        score: 1,
                        frequency_order: 0,
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        dictionary_index: 5,
                        source_term_exact_match_count: 1,
                        match_primary_reading: false,
                        max_original_text_length: 3,
                        headwords: [
                            TermHeadword {
                                index: 0,
                                term: "中",
                                reading: "じゅう",
                                sources: [
                                    TermSource {
                                        original_text: "中",
                                        transformed_text: "中",
                                        deinflected_text: "中",
                                        match_type: Exact,
                                        match_source: Term,
                                        is_primary: true,
                                    },
                                ],
                                tags: [],
                                word_classes: [],
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                id: "0197c995-e259-78f2-9fa0-09b43f375b99",
                                index: 1,
                                headword_indices: [
                                    0,
                                ],
                                dictionary: "旺文社国語辞典 第十一版 画像無し",
                                dictionary_index: 5,
                                dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                                score: 1,
                                frequency_order: 0,
                                sequences: [
                                    65340,
                                ],
                                is_primary: true,
                                tags: [],
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "‐じゅう【中】ヂユウ\n（接尾）\n① 期間を表す語に付いて
、その間ずっと続く意を添える。…の間。「一年―雨が多い」\n② 空間や範囲を表す語に付いて、その中に含まれるもの
全部の意を添える。…のうち、すべて。「家―さがす」「日本―」\n③ 集団を表す語に付いて、その成員のすべての意を 
表す。「親戚（しんせき）―」",
                                        html: None,
                                    },
                                ],
                            },
                            TermDefinition {
                                id: "0197c995-e259-78f2-9fa0-09ad6e9f6c30",
                                index: 0,
                                headword_indices: [
                                    0,
                                ],
                                dictionary: "旺文社国語辞典 第十一版 画像無し",
                                dictionary_index: 5,
                                dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                                score: 0,
                                frequency_order: 0,
                                sequences: [
                                    65338,
                                ],
                                is_primary: true,
                                tags: [],
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "じゅう【中】ヂユウ\n（字義）→ちゅう（中）",
                                        html: None,
                                    },
                                ],
                            },
                        ],
                        pronunciations: [],
                        frequencies: [],
                    },
                    TermDictionaryEntry {
                        entry_type: Term,
                        is_primary: true,
                        text_processor_rule_chain_candidates: [
                            [],
                        ],
                        inflection_rule_chain_candidates: [
                            InflectionRuleChainCandidate {
                                source: Algorithm,
                                inflection_rules: [],
                            },
                        ],
                        score: 1,
                        frequency_order: 0,
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        dictionary_index: 5,
                        source_term_exact_match_count: 1,
                        match_primary_reading: false,
                        max_original_text_length: 3,
                        headwords: [
                            TermHeadword {
                                index: 0,
                                term: "中",
                                reading: "なか",
                                sources: [
                                    TermSource {
                                        original_text: "中",
                                        transformed_text: "中",
                                        deinflected_text: "中",
                                        match_type: Exact,
                                        match_source: Term,
                                        is_primary: true,
                                    },
                                ],
                                tags: [],
                                word_classes: [],
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                id: "0197c995-e254-7903-bb1c-5a213897bb6c",
                                index: 0,
                                headword_indices: [
                                    0,
                                ],
                                dictionary: "旺文社国語辞典 第十一版 画像無し",
                                dictionary_index: 5,
                                dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                                score: 1,
                                frequency_order: 0,
                                sequences: [
                                    109042,
                                ],
                                is_primary: true,
                                tags: [],
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "なか【中】\n① （物や境などで区切られたものの）その内 
側。その内部。「家の―」「袋の―」\n⇔外(1)：外(2)\n② \n㋐ （隔たった二つのものの）あいだ。中間。\n㋑ 中央。 
まんなか。三つのもののうちの二番目。「―の兄」\n③ 限られた範囲。「クラスの―で一番背が高い」\n④ （物事が進行
している）その最中。「あらしの―を行く」\n⑤ ｟俗｠昔、遊郭を指した語。特に、東京の吉原、大阪の新町について 
言った。",
                                        html: None,
                                    },
                                ],
                            },
                        ],
                        pronunciations: [],
                        frequencies: [],
                    },
                    TermDictionaryEntry {
                        entry_type: Term,
                        is_primary: true,
                        text_processor_rule_chain_candidates: [
                            [],
                        ],
                        inflection_rule_chain_candidates: [
                            InflectionRuleChainCandidate {
                                source: Algorithm,
                                inflection_rules: [],
                            },
                        ],
                        score: 1,
                        frequency_order: 0,
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        dictionary_index: 5,
                        source_term_exact_match_count: 1,
                        match_primary_reading: false,
                        max_original_text_length: 3,
                        headwords: [
                            TermHeadword {
                                index: 0,
                                term: "中",
                                reading: "ぢゅう",
                                sources: [
                                    TermSource {
                                        original_text: "中",
                                        transformed_text: "中",
                                        deinflected_text: "中",
                                        match_type: Exact,
                                        match_source: Term,
                                        is_primary: true,
                                    },
                                ],
                                tags: [],
                                word_classes: [],
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                id: "0197c995-e2a9-79c0-b569-1df3ce806eab",
                                index: 0,
                                headword_indices: [
                                    0,
                                ],
                                dictionary: "旺文社国語辞典 第十一版 画像無し",
                                dictionary_index: 5,
                                dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                                score: 1,
                                frequency_order: 0,
                                sequences: [
                                    94704,
                                ],
                                is_primary: true,
                                tags: [],
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "‐ぢゅう【中】\n（接尾）「じゅう」と書くのが本則。 →じ
ゅう（中）",
                                        html: None,
                                    },
                                ],
                            },
                        ],
                        pronunciations: [],
                        frequencies: [],
                    },
                ],
                sentence: Sentence {
                    text: "今勉強中です",
                    offset: 0,
                },
            },
        ),
    },
    TermSearchResultsSegment {
        text: "です",
        results: Some(
            TermSearchResults {
                dictionary_entries: [
                    TermDictionaryEntry {
                        entry_type: Term,
                        is_primary: true,
                        text_processor_rule_chain_candidates: [
                            [],
                        ],
                        inflection_rule_chain_candidates: [
                            InflectionRuleChainCandidate {
                                source: Algorithm,
                                inflection_rules: [],
                            },
                        ],
                        score: 1,
                        frequency_order: 0,
                        dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                        dictionary_index: 0,
                        source_term_exact_match_count: 1,
                        match_primary_reading: false,
                        max_original_text_length: 6,
                        headwords: [
                            TermHeadword {
                                index: 0,
                                term: "です",
                                reading: "です",
                                sources: [
                                    TermSource {
                                        original_text: "です",
                                        transformed_text: "です",
                                        deinflected_text: "です",
                                        match_type: Exact,
                                        match_source: Term,
                                        is_primary: true,
                                    },
                                ],
                                tags: [],
                                word_classes: [],
                            },
                        ],
                        definitions: [
                            TermDefinition {
                                id: "0197c995-e2ab-7e43-bbcc-9cb04b65e865",
                                index: 0,
                                headword_indices: [
                                    0,
                                ],
                                dictionary: "旺文社国語辞典 第十一版 画像無し",
                                dictionary_index: 0,
                                dictionary_alias: "旺文社国語辞典 第十一版 画像無し",
                                score: 1,
                                frequency_order: 0,
                                sequences: [
                                    100734,
                                ],
                                is_primary: true,
                                tags: [],
                                entries: [
                                    TermGlossaryContentGroup {
                                        plain_text: "です\n（助動－特殊型）《デシヨ・デシ・デス・（デス） 
・○・○》\n丁寧な断定の意を表す。「ここは学校―よ」「今晩は寒い―ね」\n形容詞の丁寧形は連用形に「ございます」
を付けた「高うございます」の形が用いられたが、現在は「高いです」の形も用いられる。\n名詞および助詞（の・ほ
ど・から・など・まで・だけ・くらい・ばかり、など）に付く。未然形「でしょ」に限り、動詞・形容詞・動詞型の助
動詞・形容詞型の助動詞・特殊型の助動詞（ます・た・ぬ）の連体形に付く。連体形「です」は、ふつう助詞「ので」
「のに」を伴い、「ですので」「ですのに」となる場合にだけ用いられる。\n【変遷】「でござります」→「でござん 
す」→「であんす」→「でんす」→「でえす」→「です」と変化したとする説、「で候（そうろう）」を略した「で候（そ
う）」の転とする説など諸説ある。動詞の終止形につく「～するです」などの形は、古い言い方や方言で用いられるが
、現在の共通語では避けられ、「行くです」は「行きます」という。",
                                        html: None,
                                    },
                                ],
                            },
                        ],
                        pronunciations: [],
                        frequencies: [],
                    },
                ],
                sentence: Sentence {
                    text: "今勉強中です",
                    offset: 0,
                },
            },
        ),
    },
]
```
