use crate::dictionary_importer::*;
#[allow(unused_imports)]
use crate::structured_content::ContentMatchType;

#[test]
fn dict() {
    #[cfg(target_os = "linux")]
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    let path = std::path::Path::new("./test_dicts/四字熟語辞典オンライン");
    prepare_dictionary(path).unwrap();

    #[cfg(target_os = "linux")]
    if let Ok(report) = guard.report().build() {
        let file = std::fs::File::create("flamegraph.svg").unwrap();
        report.flamegraph(file).unwrap();
    };
}

#[test]
fn hardcoded() {
    let temp_dir = tempfile::tempdir().unwrap();
    let dir_path = temp_dir.path().to_str().unwrap();

    assert!(std::path::Path::new(&dir_path).exists());

    let json_1 = r#"
    [[
    "糸", 
    "いと", 
    "", 
    "",
    0, 
    [
      {
        "type": "structured-content",
        "content": {
          "tag": "a",
          "href": "?query=糸を引く&wildcards=off",
          "content": "糸を引く"
        }
      }
    ],
    500, 
    ""
    ]]
  "#;

    //looks janky but is valid
    let json_0 = r#"
          [
          [
            "手信号",
            "てしんごう",
            "",
            "",
            -1,
            [
              {
                "type": "structured-content",
                "content": [
                  {
                    "content": [
                      {
                        "content": [
                          "〔",
                          {
                            "content": [
                              {
                                "content": "「てん」",
                                "data": {
                                  "name": "語例"
                                },
                                "tag": "span"
                              },
                              "とも"
                            ],
                            "data": {
                              "name": "補説"
                            },
                            "tag": "span"
                          },
                          "〕"
                        ],
                        "data": {
                          "name": "補説G"
                        },
                        "tag": "div"
                      }
                    ],
                    "data": {
                      "name": "解説部"
                    },
                    "tag": "div"
                  }
                ]
              }
            ],
            95500000
        ]
        ]
        "#;

    let json_5 = r#"
    [[
    "手数",
    "てすう",
    "子",
    "",
    -1,
    [
      {
        "type": "structured-content",
        "content": {
              "tag": "a",
              "href": "?query=手数料&wildcards=off",
              "content": "手数料"
            }
      }
    ],
    500,
    ""
    ]]
  "#;

    let paths = [
        format!("{dir_path}\\term_bank_0.json"),
        format!("{dir_path}\\term_bank_1.json"),
    ];

    std::fs::write(&paths[0], json_0.as_bytes()).unwrap();
    //std::fs::write(&paths[1], json_1.as_bytes()).unwrap();

    prepare_dictionary(std::path::Path::new(dir_path)).unwrap();
}
